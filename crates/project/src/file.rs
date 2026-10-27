//! Project files (`.b4j`), module files (`.bas`) and the IDE's state files
//! (`.b4j.meta`), read so that writing them back gives the same bytes:
//! the byte order mark, every line and its line end are kept
//! (`docs/spec/project-files.md`).

use std::fmt;

/// One line, without its line end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    /// `"\r\n"`, `"\r"`, `"\n"`, or `""` for a last line without one.
    pub ending: String,
}

impl Line {
    /// Its key and value, split at the first `=`.
    pub fn key_value(&self) -> Option<(&str, &str)> {
        self.text.split_once('=')
    }
}

/// Splits `text` into lines, ending them as the lexer does: CRLF, CR or LF.
fn lines(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let len = rest.find(['\r', '\n']).unwrap_or(rest.len());
        let ending = if rest[len..].starts_with("\r\n") {
            2
        } else {
            usize::from(len < rest.len())
        };
        lines.push(Line {
            text: rest[..len].to_string(),
            ending: rest[len..len + ending].to_string(),
        });
        rest = &rest[len + ending..];
    }
    lines
}

fn strip_bom(text: &str) -> (bool, &str) {
    match text.strip_prefix('\u{feff}') {
        Some(rest) => (true, rest),
        None => (false, text),
    }
}

/// A project or module file: its header, up to the `@EndOfDesignText@`
/// line, then its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFile {
    pub bom: bool,
    /// The `Key=Value` lines before `@EndOfDesignText@`.
    pub lines: Vec<Line>,
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
        let (bom, text) = strip_bom(text);
        let len = ferro_syntax::lexer::header_len(text)?;
        let mut lines = lines(&text[..len]);
        let end = lines.pop().expect("a header ends with its marker line");
        debug_assert_eq!(end.text, "@EndOfDesignText@");
        Some(HeaderFile {
            bom,
            lines,
            end_ending: end.ending,
            code: text[len..].to_string(),
        })
    }

    /// The value of the first line with `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// Every `Key=Value` line, in order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.lines.iter().filter_map(Line::key_value)
    }

    /// The values of `<prefix><n>` keys (`Library1`, `Library2`…), by `n`.
    pub fn numbered(&self, prefix: &str) -> Vec<(u32, &str)> {
        let mut found: Vec<(u32, &str)> = self
            .entries()
            .filter_map(|(key, value)| Some((number(key, prefix)?, value)))
            .collect();
        found.sort_by_key(|(n, _)| *n);
        found
    }
}

impl fmt::Display for HeaderFile {
    /// The file, byte for byte as it was read.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bom {
            f.write_str("\u{feff}")?;
        }
        for line in &self.lines {
            write!(f, "{}{}", line.text, line.ending)?;
        }
        write!(f, "@EndOfDesignText@{}{}", self.end_ending, self.code)
    }
}

/// `n` in a key `<prefix><n>`: `Library12` with `Library` gives 12.
pub fn number(key: &str, prefix: &str) -> Option<u32> {
    let digits = key.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// A `<project>.b4j.meta` file: the IDE's state (bookmarks, breakpoints,
/// folded regions, the selected build), `Key=Value` lines with no
/// `@EndOfDesignText@`. Nothing in it changes the program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaFile {
    pub bom: bool,
    pub lines: Vec<Line>,
}

impl MetaFile {
    pub fn parse(text: &str) -> MetaFile {
        let (bom, text) = strip_bom(text);
        MetaFile {
            bom,
            lines: lines(text),
        }
    }

    /// The value of the first line with `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines
            .iter()
            .filter_map(Line::key_value)
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }
}

impl fmt::Display for MetaFile {
    /// The file, byte for byte as it was read.
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
    fn reads_keys_and_numbered_keys() {
        let file = HeaderFile::parse(
            "Library2=jstringutils\nIconFile=\nBuild1=Default,b4j.example\nLibrary10=json\nLibrary1=jcore\nLibraryX=no\n@EndOfDesignText@\n",
        )
        .unwrap();
        assert_eq!(file.get("IconFile"), Some(""));
        assert_eq!(file.get("Build1"), Some("Default,b4j.example"));
        assert_eq!(file.get("Group"), None);
        assert_eq!(
            file.numbered("Library"),
            [(1, "jcore"), (2, "jstringutils"), (10, "json")]
        );
        assert_eq!(number("Module12", "Module"), Some(12));
        assert_eq!(number("Module", "Module"), None);
        assert_eq!(number("FileGroup1", "File"), None);
        assert_eq!(number("Module99999999999", "Module"), None);
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
