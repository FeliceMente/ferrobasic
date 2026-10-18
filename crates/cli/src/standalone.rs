//! Standalone binaries (`docs/spec/cli.md`, "Standalone binary format").
//!
//! `ferro build` embeds the checked program, as a *package*, in a copy of the
//! running `ferro` executable:
//!
//! ```text
//! package = [ payload ][ payload length: u64 LE ][ "FERROBASIC\0PKG" ][ version: u16 LE ]
//! ```
//!
//! - Linux and Windows: the package is appended to the executable. At
//!   start-up `ferro` reads the trailer at the end of its own file.
//! - macOS: appended bytes cannot be code-signed, so `libsui` stores the
//!   package in a Mach-O section (Apple Silicon) or inside `__LINKEDIT`
//!   (Intel), and finds it again at start-up.
//!
//! When a package is present, `ferro` runs it instead of its CLI.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
#[cfg(not(target_os = "macos"))]
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Mach-O section holding the package on macOS (at most 16 bytes).
#[cfg(target_os = "macos")]
const SECTION: &str = "__ferro";

/// Marks a standalone binary.
pub const MAGIC: &[u8; 14] = b"FERROBASIC\0PKG";
/// Format version written by this `ferro`.
pub const VERSION: u16 = 1;
/// Length of the trailer: payload length, magic, version.
pub const TRAILER_LEN: usize = 8 + MAGIC.len() + 2;

/// One file of the embedded program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Why an executable's embedded program cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadError {
    UnknownVersion(u16),
    /// The lengths in the trailer or the payload do not fit the file.
    Corrupt(&'static str),
}

impl std::fmt::Display for PayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadError::UnknownVersion(v) => {
                write!(f, "embedded program has unknown format version {v}")
            }
            PayloadError::Corrupt(what) => write!(f, "embedded program is corrupt: {what}"),
        }
    }
}

/// Serialises entries as: entry count (u32 LE), then for each entry its name
/// and its bytes, each a length (u32 LE) followed by the data.
pub fn encode(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        for part in [entry.name.as_bytes(), entry.bytes.as_slice()] {
            out.extend_from_slice(&(part.len() as u32).to_le_bytes());
            out.extend_from_slice(part);
        }
    }
    out
}

/// The inverse of [`encode`]; the whole input must be consumed.
pub fn decode(mut data: &[u8]) -> Result<Vec<Entry>, PayloadError> {
    fn take<'a>(data: &mut &'a [u8], n: usize) -> Result<&'a [u8], PayloadError> {
        if data.len() < n {
            return Err(PayloadError::Corrupt("truncated entry"));
        }
        let (head, rest) = data.split_at(n);
        *data = rest;
        Ok(head)
    }
    fn u32_at(data: &mut &[u8]) -> Result<usize, PayloadError> {
        let bytes = take(data, 4)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()) as usize)
    }
    let count = u32_at(&mut data)?;
    let mut entries = Vec::new();
    for _ in 0..count {
        let len = u32_at(&mut data)?;
        let name = std::str::from_utf8(take(&mut data, len)?)
            .map_err(|_| PayloadError::Corrupt("entry name is not UTF-8"))?
            .to_string();
        let len = u32_at(&mut data)?;
        let bytes = take(&mut data, len)?.to_vec();
        entries.push(Entry { name, bytes });
    }
    if !data.is_empty() {
        return Err(PayloadError::Corrupt("trailing bytes after the last entry"));
    }
    Ok(entries)
}

/// The trailer for a payload of `len` bytes.
pub fn trailer(len: u64) -> [u8; TRAILER_LEN] {
    let mut out = [0u8; TRAILER_LEN];
    out[..8].copy_from_slice(&len.to_le_bytes());
    out[8..8 + MAGIC.len()].copy_from_slice(MAGIC);
    out[8 + MAGIC.len()..].copy_from_slice(&VERSION.to_le_bytes());
    out
}

/// Reads a trailer: `Ok(None)` when there is none (a plain `ferro`), the
/// payload length otherwise. `file_len` is the length of the whole file.
pub fn parse_trailer(tail: &[u8; TRAILER_LEN], file_len: u64) -> Result<Option<u64>, PayloadError> {
    if &tail[8..8 + MAGIC.len()] != MAGIC {
        return Ok(None);
    }
    let version = u16::from_le_bytes(tail[8 + MAGIC.len()..].try_into().unwrap());
    if version != VERSION {
        return Err(PayloadError::UnknownVersion(version));
    }
    let len = u64::from_le_bytes(tail[..8].try_into().unwrap());
    if len > file_len.saturating_sub(TRAILER_LEN as u64) {
        return Err(PayloadError::Corrupt("payload length exceeds the file"));
    }
    Ok(Some(len))
}

/// Payload and trailer for `entries`: what a standalone binary embeds.
pub fn package(entries: &[Entry]) -> Vec<u8> {
    let mut bytes = encode(entries);
    let len = bytes.len() as u64;
    bytes.extend_from_slice(&trailer(len));
    bytes
}

/// The inverse of [`package`]: `data` must be exactly one package, as
/// `libsui` returns it on macOS.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn unpack(data: &[u8]) -> Result<Vec<Entry>, PayloadError> {
    let Some(split) = data.len().checked_sub(TRAILER_LEN) else {
        return Err(PayloadError::Corrupt("package shorter than its trailer"));
    };
    let (payload, tail) = data.split_at(split);
    match parse_trailer(tail.try_into().unwrap(), data.len() as u64)? {
        Some(len) if len == payload.len() as u64 => decode(payload),
        Some(_) => Err(PayloadError::Corrupt("payload length does not match")),
        None => Err(PayloadError::Corrupt("missing trailer")),
    }
}

/// The program embedded in the running executable `exe`, if there is one.
#[cfg(not(target_os = "macos"))]
pub fn embedded_program(exe: &Path) -> io::Result<Result<Option<Vec<Entry>>, PayloadError>> {
    Ok(read_embedded(exe)?.map(|embedded| embedded.entries))
}

/// The program embedded in the running executable, if there is one.
#[cfg(target_os = "macos")]
pub fn embedded_program(_exe: &Path) -> io::Result<Result<Option<Vec<Entry>>, PayloadError>> {
    Ok(match libsui::find_section(SECTION)? {
        Some(data) => unpack(data).map(Some),
        None => Ok(None),
    })
}

/// The bytes of a standalone executable: `exe` with `entries` embedded.
/// A standalone `exe` contributes only its executable part.
#[cfg(not(target_os = "macos"))]
pub fn standalone_bytes(exe: &Path, entries: &[Entry]) -> io::Result<Vec<u8>> {
    let exe_len = match read_embedded(exe)? {
        Ok(embedded) => embedded.exe_len,
        Err(error) => return Err(io::Error::other(error.to_string())),
    };
    let mut bytes = Vec::new();
    File::open(exe)?.take(exe_len).read_to_end(&mut bytes)?;
    bytes.extend_from_slice(&package(entries));
    Ok(bytes)
}

/// The bytes of a standalone executable: `exe` with `entries` embedded by
/// `libsui`. On Apple Silicon it also signs the result ad hoc; on Intel the
/// caller signs the written file with `codesign`.
#[cfg(target_os = "macos")]
pub fn standalone_bytes(exe: &Path, entries: &[Entry]) -> io::Result<Vec<u8>> {
    let sui = |e: libsui::Error| io::Error::other(e.to_string());
    #[cfg(not(target_arch = "aarch64"))]
    let _reserved = ReservedLibsuiTemp::create()?;
    let macho = libsui::Macho::from(std::fs::read(exe)?)
        .map_err(sui)?
        .write_section(SECTION, package(entries))
        .map_err(sui)?;
    let mut bytes = Vec::new();
    #[cfg(target_arch = "aarch64")]
    macho.build_and_sign(&mut bytes).map_err(sui)?;
    #[cfg(not(target_arch = "aarch64"))]
    macho.build(&mut bytes).map_err(sui)?;
    Ok(bytes)
}

/// On Intel, `libsui` 0.16.4 copies the executable to
/// `temp_dir()/sui_tmp_<pid>` with `File::create`, which follows symlinks and
/// truncates. With a shared `TMPDIR` such as `/tmp`, another user could plant
/// a symlink there and make `ferro build` overwrite a file of the builder.
/// Creating the path first with `create_new` (which refuses symlinks and
/// existing files) makes `libsui` open our own regular file; in a sticky
/// directory nobody else can replace it.
#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
struct ReservedLibsuiTemp(PathBuf);

#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
impl ReservedLibsuiTemp {
    fn create() -> io::Result<Self> {
        let path = std::env::temp_dir().join(format!("sui_tmp_{}", std::process::id()));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!("cannot create temporary file {}: {e}", path.display()),
                )
            })?;
        Ok(Self(path))
    }
}

#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
impl Drop for ReservedLibsuiTemp {
    /// `libsui` removes the file itself; this covers its error paths.
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// What the end of an executable holds.
#[cfg(not(target_os = "macos"))]
pub struct Embedded {
    /// Length of the executable without payload and trailer.
    pub exe_len: u64,
    /// `None` for a plain `ferro`.
    pub entries: Option<Vec<Entry>>,
}

/// Inspects `path`: finds a trailer and decodes the payload if there is one.
#[cfg(not(target_os = "macos"))]
pub fn read_embedded(path: &Path) -> io::Result<Result<Embedded, PayloadError>> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < TRAILER_LEN as u64 {
        return Ok(Ok(Embedded {
            exe_len: file_len,
            entries: None,
        }));
    }
    let mut tail = [0u8; TRAILER_LEN];
    file.seek(SeekFrom::End(-(TRAILER_LEN as i64)))?;
    file.read_exact(&mut tail)?;
    let len = match parse_trailer(&tail, file_len) {
        Ok(Some(len)) => len,
        Ok(None) => {
            return Ok(Ok(Embedded {
                exe_len: file_len,
                entries: None,
            }));
        }
        Err(e) => return Ok(Err(e)),
    };
    let exe_len = file_len - TRAILER_LEN as u64 - len;
    file.seek(SeekFrom::Start(exe_len))?;
    let mut payload = vec![0u8; len as usize];
    file.read_exact(&mut payload)?;
    Ok(decode(&payload).map(|entries| Embedded {
        exe_len,
        entries: Some(entries),
    }))
}

/// Writes `bytes` (from [`standalone_bytes`]) to `out`, replacing it, with
/// the executable permission bits on Unix. `finish` runs on the complete
/// staged file before it replaces `out` (Intel Macs sign it there); if
/// anything fails, `out` is left as it was.
pub fn write_standalone(
    bytes: &[u8],
    out: &Path,
    finish: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    // Stage next to the target, then rename: never leave a half-written
    // binary at `out`, and work even when `out` is a running executable.
    let (mut file, staged) = create_staging(out)?;
    let result = (|| {
        file.write_all(bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o755))?;
        }
        drop(file);
        finish(&staged)?;
        std::fs::rename(&staged, out)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&staged);
    }
    result
}

/// Creates a new, unique file next to `out`. `create_new` never opens an
/// existing file or follows a symlink, so concurrent builds and stray files
/// with the same name are never overwritten.
fn create_staging(out: &Path) -> io::Result<(File, PathBuf)> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = match out.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let name = out.file_name().unwrap_or(out.as_os_str()).to_string_lossy();
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let staged = dir.join(format!(".{name}.{}.{n}.ferro-tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
        {
            Ok(file) => return Ok((file, staged)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && n < 1000 => continue,
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Entry> {
        vec![
            Entry {
                name: "main.bas".into(),
                bytes: b"Sub AppStart (Args() As String)\r\nEnd Sub\r\n".to_vec(),
            },
            Entry {
                name: "é.txt".into(),
                bytes: vec![],
            },
        ]
    }

    #[test]
    fn payload_round_trips() {
        assert_eq!(decode(&encode(&sample())).unwrap(), sample());
        assert_eq!(decode(&encode(&[])).unwrap(), vec![]);
    }

    #[test]
    fn truncated_or_padded_payloads_are_corrupt() {
        let data = encode(&sample());
        for cut in 0..data.len() {
            assert!(
                decode(&data[..cut]).is_err(),
                "prefix of {cut} bytes accepted"
            );
        }
        let mut padded = data.clone();
        padded.push(0);
        assert!(decode(&padded).is_err());
        let mut huge = data;
        huge[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&huge).is_err());
    }

    #[test]
    fn trailers() {
        let t = trailer(10);
        assert_eq!(parse_trailer(&t, 100), Ok(Some(10)));
        // A plain executable: no magic.
        assert_eq!(parse_trailer(&[0; TRAILER_LEN], 100), Ok(None));
        // Payload longer than the file.
        assert!(matches!(
            parse_trailer(&t, 20),
            Err(PayloadError::Corrupt(_))
        ));
        let mut future = trailer(10);
        future[TRAILER_LEN - 2..].copy_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            parse_trailer(&future, 100),
            Err(PayloadError::UnknownVersion(7))
        );
    }

    #[test]
    fn packages_round_trip() {
        assert_eq!(unpack(&package(&sample())).unwrap(), sample());
        let data = package(&sample());
        for cut in 0..data.len() {
            assert!(
                unpack(&data[..cut]).is_err(),
                "prefix of {cut} bytes accepted"
            );
        }
        // Bytes before the payload make the length disagree.
        let mut prefixed = vec![0];
        prefixed.extend_from_slice(&data);
        assert!(unpack(&prefixed).is_err());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn write_then_read_back() {
        let dir =
            std::env::temp_dir().join(format!("ferro-standalone-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("fake-ferro");
        std::fs::write(&exe, b"\x7fELF not really an executable").unwrap();
        let out = dir.join("prog");
        let bytes = standalone_bytes(&exe, &sample()).unwrap();
        write_standalone(&bytes, &out, |_| Ok(())).unwrap();
        let embedded = read_embedded(&out).unwrap().unwrap();
        assert_eq!(embedded.exe_len, 29);
        assert_eq!(embedded.entries, Some(sample()));
        assert_eq!(embedded_program(&out).unwrap().unwrap(), Some(sample()));
        // The plain "executable" has no payload.
        assert_eq!(embedded_program(&exe).unwrap().unwrap(), None);
        // Rebuilding from a standalone binary keeps only its executable part.
        let again = dir.join("again");
        let bytes = standalone_bytes(&out, &sample()[..1]).unwrap();
        write_standalone(&bytes, &again, |_| Ok(())).unwrap();
        let read = read_embedded(&again).unwrap().unwrap();
        assert_eq!(read.exe_len, 29);
        assert_eq!(read.entries.unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn staging_never_touches_other_files() {
        let dir =
            std::env::temp_dir().join(format!("ferro-standalone-stage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("prog");
        std::fs::write(&out, b"previous").unwrap();
        // A file at the old fixed staging name is left alone.
        let stray = dir.join("prog.ferro-tmp");
        std::fs::write(&stray, b"unrelated").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&stray, dir.join(".prog.ferro-tmp")).unwrap();

        // A failing finishing step (codesign on Intel Macs) keeps the old
        // output and removes the staged file.
        let err = write_standalone(b"new", &out, |staged| {
            assert!(staged.exists() && staged != out);
            Err(io::Error::other("signing failed"))
        })
        .unwrap_err();
        assert_eq!(err.to_string(), "signing failed");
        assert_eq!(std::fs::read(&out).unwrap(), b"previous");

        write_standalone(b"new", &out, |_| Ok(())).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"new");
        assert_eq!(std::fs::read(&stray).unwrap(), b"unrelated");
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        let mut expected = vec!["prog", "prog.ferro-tmp"];
        if cfg!(unix) {
            expected.insert(0, ".prog.ferro-tmp");
        }
        assert_eq!(names, expected);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
