//! Project files (`.b4j`), module files (`.bas`) and the IDE's state files
//! (`.b4j.meta`), read so that writing them back gives the same bytes:
//! the byte order mark, every line and its line end are kept
//! (`docs/spec/project-files.md`).

use std::cmp::Ordering;
use std::fmt;

/// One line, without its line end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    /// `"\r\n"`, `"\r"`, `"\n"`, or `""` for a last line without one.
    pub ending: String,
    /// Its byte offset in the whole file, byte order mark included.
    pub offset: usize,
}

impl Line {
    /// Its key and value, split at the first `=`.
    pub fn key_value(&self) -> Option<(&str, &str)> {
        self.text.split_once('=')
    }
}

/// Lines after an optional byte order mark: a `.b4j.meta` file, or the
/// header of a project or module file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub bom: bool,
    pub lines: Vec<Line>,
}

/// A `<project>.b4j.meta` file: the IDE's state (bookmarks, breakpoints,
/// folded regions, the selected build), `Key=Value` lines with no
/// `@EndOfDesignText@`. Nothing in it changes the program.
pub type MetaFile = Lines;

impl Lines {
    /// Reads a whole file as lines.
    pub fn parse(text: &str) -> Lines {
        let (bom, text) = strip_bom(text);
        Lines {
            bom,
            lines: split_lines(text, bom_len(bom)),
        }
    }

    /// Every `Key=Value` line, in order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.lines.iter().filter_map(Line::key_value)
    }

    /// The value of the first line with `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// The values of `<prefix><n>` keys (`Library1`, `Library2`…), in the
    /// order of `n`, with `n`'s digits (without leading zeros). A number of
    /// any length counts.
    pub fn numbered(&self, prefix: &str) -> Vec<(&str, &str)> {
        let mut found: Vec<(&str, &str)> = self
            .entries()
            .filter_map(|(key, value)| Some((number(key, prefix)?, value)))
            .collect();
        found.sort_by(|a, b| compare_numbers(a.0, b.0));
        found
    }
}

impl fmt::Display for Lines {
    /// The lines, byte for byte as they were read.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bom {
            f.write_str("\u{feff}")?;
        }
        for line in &self.lines {
            write!(f, "{}{}", line.text, line.ending)?;
        }
        Ok(())
    }
}

fn strip_bom(text: &str) -> (bool, &str) {
    match text.strip_prefix('\u{feff}') {
        Some(rest) => (true, rest),
        None => (false, text),
    }
}

fn bom_len(bom: bool) -> usize {
    if bom { '\u{feff}'.len_utf8() } else { 0 }
}

/// Splits `text` into lines, ending them as the lexer does: CRLF, CR or LF.
/// `start` is the offset of `text` in the whole file.
fn split_lines(text: &str, start: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        let len = rest.find(['\r', '\n']).unwrap_or(rest.len());
        let ending = if rest[len..].starts_with("\r\n") {
            2
        } else {
            usize::from(len < rest.len())
        };
        lines.push(Line {
            text: rest[..len].to_string(),
            ending: rest[len..len + ending].to_string(),
            offset: start + offset,
        });
        offset += len + ending;
    }
    lines
}

/// A project or module file: its header, up to the `@EndOfDesignText@`
/// line, then its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFile {
    /// The byte order mark and the `Key=Value` lines before
    /// `@EndOfDesignText@`.
    pub header: Lines,
    /// The line end after `@EndOfDesignText@`.
    pub end_ending: String,
    /// Everything after the header, as it is.
    pub code: String,
}

impl HeaderFile {
    /// Reads `text`, or `None` when it does not start (after a byte order
    /// mark) with a header: the lexer's own rule, so that the two always
    /// agree on where the code starts.
    pub fn parse(text: &str) -> Option<HeaderFile> {
        let (bom, rest) = strip_bom(text);
        let len = ferro_syntax::lexer::header_len(rest)?;
        let mut lines = split_lines(&rest[..len], bom_len(bom));
        let end = lines.pop().expect("a header ends with its marker line");
        debug_assert_eq!(end.text, "@EndOfDesignText@");
        Some(HeaderFile {
            header: Lines { bom, lines },
            end_ending: end.ending,
            code: rest[len..].to_string(),
        })
    }

    /// The value of the first header line with `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.header.get(key)
    }

    /// See [`Lines::numbered`].
    pub fn numbered(&self, prefix: &str) -> Vec<(&str, &str)> {
        self.header.numbered(prefix)
    }
}

impl fmt::Display for HeaderFile {
    /// The file, byte for byte as it was read.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@EndOfDesignText@{}{}",
            self.header, self.end_ending, self.code
        )
    }
}

/// The digits `n` of a key `<prefix><n>`, without leading zeros:
/// `Library12` with `Library` gives `"12"`, `Library007` gives `"7"`.
pub fn number<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let digits = key.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let trimmed = digits.trim_start_matches('0');
    Some(if trimmed.is_empty() { "0" } else { trimmed })
}

/// Orders the digits [`number`] gives by their value, whatever their length.
pub fn compare_numbers(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_file_round_trips() {
        for text in [
            "\u{feff}AppType=StandardJava\r\nLibrary1=jcore\r\n@EndOfDesignText@\r\nSub X\r\nEnd Sub\r\n",
            "Version=2.18\nAppType=StandardJava\nIconFile=\n@EndOfDesignText@\nSub X\n",
            "A=1\rB=2\r\n@EndOfDesignText@",
            "A=1\n@EndOfDesignText@\n",
            "Type=StaticCode\n@EndOfDesignText@\r\n'no line end at the end",
        ] {
            let file = HeaderFile::parse(text).unwrap();
            assert_eq!(file.to_string(), text);
        }
        assert_eq!(HeaderFile::parse("Sub X\nEnd Sub\n"), None);
        assert_eq!(HeaderFile::parse(""), None);
    }

    #[test]
    fn lines_know_their_offsets() {
        let text = "\u{feff}A=1\r\nB=2\rC=3\n@EndOfDesignText@\n";
        let file = HeaderFile::parse(text).unwrap();
        let at: Vec<&str> = file
            .header
            .lines
            .iter()
            .map(|l| &text[l.offset..l.offset + l.text.len()])
            .collect();
        assert_eq!(at, ["A=1", "B=2", "C=3"]);
        assert_eq!(file.header.lines[0].offset, 3);
    }

    #[test]
    fn reads_keys_and_numbered_keys() {
        let file = HeaderFile::parse(
            "Library2=jstringutils\nIconFile=\nBuild1=Default,b4j.example\nLibrary10=json\nLibrary1=jcore\nLibraryX=no\nLibrary99999999999=jfx\n@EndOfDesignText@\n",
        )
        .unwrap();
        assert_eq!(file.get("IconFile"), Some(""));
        assert_eq!(file.get("Build1"), Some("Default,b4j.example"));
        assert_eq!(file.get("Group"), None);
        assert_eq!(
            file.numbered("Library"),
            [
                ("1", "jcore"),
                ("2", "jstringutils"),
                ("10", "json"),
                ("99999999999", "jfx")
            ]
        );
        assert_eq!(number("Module12", "Module"), Some("12"));
        assert_eq!(number("Module007", "Module"), Some("7"));
        assert_eq!(number("Module0", "Module"), Some("0"));
        assert_eq!(number("Module", "Module"), None);
        assert_eq!(number("FileGroup1", "File"), None);
        assert_eq!(
            number("Module99999999999999999999", "Module"),
            Some("99999999999999999999")
        );
        assert_eq!(compare_numbers("9", "10"), Ordering::Less);
        assert_eq!(compare_numbers("12", "12"), Ordering::Equal);
    }

    #[test]
    fn a_meta_file_round_trips() {
        for text in [
            "\u{feff}ModuleBookmarks0=\r\nModuleBookmarks1=31\r\nSelectedBuild=0\r\n",
            "VisibleModules=1",
            "",
            "A=1\n\nno equals sign\r",
        ] {
            let meta = MetaFile::parse(text);
            assert_eq!(meta.to_string(), text);
        }
        let meta = MetaFile::parse("ModuleBookmarks1=31\nSelectedBuild=0\n");
        assert_eq!(meta.get("SelectedBuild"), Some("0"));
        assert_eq!(meta.get("ModuleBookmarks1"), Some("31"));
    }
}
