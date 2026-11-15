//! Standalone binaries (`docs/spec/cli.md`, "Standalone binary format").
//!
//! `ferro bundle` embeds the checked program, as a *package*, in a copy of the
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
    macho_header_fits(&bytes).map_err(io::Error::other)?;
    Ok(bytes)
}

/// `MH_MAGIC_64`: a thin 64-bit Mach-O file.
const MH_MAGIC_64: u32 = 0xfeed_facf;
/// Length of a 64-bit Mach-O header.
const MACH_HEADER_64_LEN: usize = 32;
/// `LC_SEGMENT_64`, and the lengths of its command and of one section.
const LC_SEGMENT_64: u32 = 0x19;
const SEGMENT_COMMAND_64_LEN: usize = 72;
const SECTION_64_LEN: usize = 80;

/// Checks that the load commands of the 64-bit Mach-O file `bytes` end
/// before the first byte of any section. `libsui` adds a load command
/// without checking for room: when the executable has too little header
/// padding, the new command overwrites the start of `__text`, and the
/// binary crashes at start-up (issue #131).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn macho_header_fits(bytes: &[u8]) -> Result<(), String> {
    let u32_at = |at: usize| -> Result<u32, String> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| "the Mach-O header is truncated".to_owned())
    };
    if u32_at(0)? != MH_MAGIC_64 {
        return Err("not a 64-bit Mach-O executable".into());
    }
    let ncmds = u32_at(16)? as usize;
    let commands_end = MACH_HEADER_64_LEN + u32_at(20)? as usize;
    let mut first_section = usize::MAX;
    let mut at = MACH_HEADER_64_LEN;
    for _ in 0..ncmds {
        let (cmd, size) = (u32_at(at)?, u32_at(at + 4)? as usize);
        if size == 0 || at + size > commands_end {
            return Err("the Mach-O load commands are malformed".into());
        }
        if cmd == LC_SEGMENT_64 {
            let nsects = u32_at(at + 64)? as usize;
            for s in 0..nsects {
                let section = at + SEGMENT_COMMAND_64_LEN + s * SECTION_64_LEN;
                if section + SECTION_64_LEN > at + size {
                    return Err("the Mach-O load commands are malformed".into());
                }
                // Zero-fill sections have no bytes in the file (offset 0).
                let offset = u32_at(section + 48)? as usize;
                if offset != 0 {
                    first_section = first_section.min(offset);
                }
            }
        }
        at += size;
    }
    if commands_end > first_section {
        return Err(format!(
            "the embedded program does not fit: the Mach-O load commands would end at \
             byte {commands_end}, past the first section at byte {first_section}; this \
             `ferro` was linked with too little header space (rebuild it with \
             `-Wl,-headerpad,0x1000`)"
        ));
    }
    Ok(())
}

/// On Intel, `libsui` 0.16.4 copies the executable to
/// `temp_dir()/sui_tmp_<pid>` with `File::create`, which follows symlinks and
/// truncates. With a shared `TMPDIR` such as `/tmp`, another user could plant
/// a symlink there and make `ferro bundle` overwrite a file of the builder.
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

    /// A fresh directory for one test, removed when dropped, also when the
    /// test fails.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir()
                .join(format!("ferro-standalone-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl std::ops::Deref for TempDir {
        type Target = std::path::Path;
        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

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

    /// A 64-bit Mach-O header with one `LC_SEGMENT_64` holding sections at
    /// `offsets`, then zeros up to `len` bytes.
    fn macho(offsets: &[u32], len: usize) -> Vec<u8> {
        let cmdsize = SEGMENT_COMMAND_64_LEN + offsets.len() * SECTION_64_LEN;
        let mut b = vec![0u8; len.max(MACH_HEADER_64_LEN + cmdsize)];
        let put =
            |b: &mut Vec<u8>, at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 0, MH_MAGIC_64);
        put(&mut b, 16, 1);
        put(&mut b, 20, cmdsize as u32);
        put(&mut b, 32, LC_SEGMENT_64);
        put(&mut b, 36, cmdsize as u32);
        put(&mut b, 32 + 64, offsets.len() as u32);
        for (i, offset) in offsets.iter().enumerate() {
            put(
                &mut b,
                32 + SEGMENT_COMMAND_64_LEN + i * SECTION_64_LEN + 48,
                *offset,
            );
        }
        b
    }

    #[test]
    fn load_commands_must_end_before_the_first_section() {
        // One segment with two sections: the commands end at 32 + 72 + 160.
        assert_eq!(macho_header_fits(&macho(&[264, 4096], 4096)), Ok(()));
        assert_eq!(macho_header_fits(&macho(&[4096, 2368], 4096)), Ok(()));
        // Issue #131: the first section starts 80 bytes too early.
        let overlap = macho_header_fits(&macho(&[184, 4096], 4096)).unwrap_err();
        assert!(
            overlap.contains("byte 264, past the first section at byte 184"),
            "{overlap}"
        );
        // A zero-fill section has no bytes in the file.
        assert_eq!(macho_header_fits(&macho(&[0, 4096], 4096)), Ok(()));
        // No section at all: nothing to overlap.
        assert_eq!(macho_header_fits(&macho(&[], 512)), Ok(()));
    }

    #[test]
    fn malformed_mach_o_headers_are_errors() {
        assert!(macho_header_fits(b"\x7fELF").is_err());
        assert!(macho_header_fits(&MH_MAGIC_64.to_le_bytes()).is_err());
        let mut zero_size = macho(&[4096], 4096);
        zero_size[36..40].copy_from_slice(&0u32.to_le_bytes());
        assert!(macho_header_fits(&zero_size).is_err());
        // More sections than the command holds.
        let mut too_many = macho(&[4096], 4096);
        too_many[32 + 64..32 + 68].copy_from_slice(&2u32.to_le_bytes());
        assert!(macho_header_fits(&too_many).is_err());
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
        let dir = TempDir::new("unit");
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
    }

    #[test]
    fn staging_never_touches_other_files() {
        let dir = TempDir::new("stage");
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
        let mut names: Vec<_> = std::fs::read_dir(&*dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        let mut expected = vec!["prog", "prog.ferro-tmp"];
        if cfg!(unix) {
            expected.insert(0, ".prog.ferro-tmp");
        }
        assert_eq!(names, expected);
    }
}
