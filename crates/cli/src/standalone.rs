//! Standalone binaries (`docs/spec/cli.md`, "Standalone binary format").
//!
//! `ferro build` writes a copy of the running `ferro` executable followed by
//! the checked program and a trailer:
//!
//! ```text
//! [ ferro executable ][ payload ][ payload length: u64 LE ][ "FERROBASIC\0PKG" ][ version: u16 LE ]
//! ```
//!
//! At start-up `ferro` looks for the trailer at the end of its own file; when
//! present it runs the embedded program instead of its CLI.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

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

/// What the end of an executable holds.
pub struct Embedded {
    /// Length of the executable without payload and trailer.
    pub exe_len: u64,
    /// `None` for a plain `ferro`.
    pub entries: Option<Vec<Entry>>,
}

/// Inspects `path`: finds a trailer and decodes the payload if there is one.
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

/// Writes `exe[..exe_len]`, the payload and the trailer to `out`, replacing
/// it, with the executable permission bits on Unix.
pub fn write_standalone(exe: &Path, exe_len: u64, entries: &[Entry], out: &Path) -> io::Result<()> {
    let mut base = Vec::new();
    File::open(exe)?.take(exe_len).read_to_end(&mut base)?;
    let payload = encode(entries);
    let mut bytes = base;
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&trailer(payload.len() as u64));
    // Write next to the target, then rename: never leave a half-written
    // binary at `out`, and work even when `out` is a running executable.
    let tmp = out.with_extension("ferro-tmp");
    std::fs::write(&tmp, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, out).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
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
    fn write_then_read_back() {
        let dir =
            std::env::temp_dir().join(format!("ferro-standalone-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("fake-ferro");
        std::fs::write(&exe, b"\x7fELF not really an executable").unwrap();
        let out = dir.join("prog");
        write_standalone(&exe, 10, &sample(), &out).unwrap();
        let embedded = read_embedded(&out).unwrap().unwrap();
        assert_eq!(embedded.exe_len, 10);
        assert_eq!(embedded.entries, Some(sample()));
        // The plain "executable" has no payload.
        let plain = read_embedded(&exe).unwrap().unwrap();
        assert!(plain.entries.is_none());
        // Rebuilding from a standalone binary keeps only its executable part.
        let again = dir.join("again");
        write_standalone(&out, embedded.exe_len, &sample()[..1], &again).unwrap();
        let read = read_embedded(&again).unwrap().unwrap();
        assert_eq!(read.exe_len, 10);
        assert_eq!(read.entries.unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
