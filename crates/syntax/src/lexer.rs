//! Lexer: splits source text into tokens that cover every byte.
//!
//! Concatenating the token texts gives back the input exactly; the parser
//! relies on this for the lossless CST.

use crate::SyntaxKind::{self, *};

/// One token: its kind and its length in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    pub len: u32,
}

/// A lexical error, at a byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub offset: u32,
    pub message: String,
}

/// Tokenises `text`. Never fails: problems become `ERROR_TOKEN`s or errors.
pub fn lex(text: &str) -> (Vec<Token>, Vec<LexError>) {
    let mut lexer = Lexer {
        src: text,
        pos: 0,
        tokens: Vec::new(),
        errors: Vec::new(),
        line_start: true,
        smart_depth: 0,
        reported_too_deep: false,
        continued_comment: false,
    };
    lexer.prologue();
    while !lexer.at_end() {
        lexer.code_token(false);
    }
    (lexer.tokens, lexer.errors)
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
    /// Only whitespace since the last line break: a `#` starts a directive.
    line_start: bool,
    /// Smart strings open around the current position, each inside a
    /// placeholder of the one before.
    smart_depth: u32,
    /// The too-deep error is reported once per file.
    reported_too_deep: bool,
    /// A comment follows a line continuation: the line break after it is
    /// part of the continuation.
    continued_comment: bool,
}

/// Deepest nesting of smart strings (`$"${$"..."$}"$`). Smart strings are
/// lexed recursively, so an unbounded depth would overflow the stack.
const MAX_SMART_NESTING: u32 = 100;

impl Lexer<'_> {
    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn push(&mut self, kind: SyntaxKind, len: usize) {
        debug_assert!(len > 0, "empty {kind:?} token");
        self.tokens.push(Token {
            kind,
            len: len as u32,
        });
        self.pos += len;
        match kind {
            NEWLINE | HEADER | BOM => self.line_start = true,
            WHITESPACE => {}
            _ => self.line_start = false,
        }
    }

    fn error(&mut self, offset: usize, message: impl Into<String>) {
        self.errors.push(LexError {
            offset: offset as u32,
            message: message.into(),
        });
    }

    /// Byte length of the prefix of the rest whose chars satisfy `pred`.
    fn take_while(&self, pred: impl Fn(char) -> bool) -> usize {
        self.rest()
            .char_indices()
            .find(|&(_, c)| !pred(c))
            .map_or(self.rest().len(), |(i, _)| i)
    }

    /// Optional BOM, then an optional `Key=Value` header up to `@EndOfDesignText@`.
    fn prologue(&mut self) {
        if self.rest().starts_with('\u{feff}') {
            self.push(BOM, '\u{feff}'.len_utf8());
        }
        if let Some(len) = header_len(self.rest()) {
            self.push(HEADER, len);
        }
    }

    /// Lexes one token in code. Inside a placeholder, a `}` is left for the
    /// caller and `false` is returned.
    fn code_token(&mut self, in_placeholder: bool) -> bool {
        let rest = self.rest();
        let c = self.peek().expect("not at end");
        match c {
            ' ' | '\t' | '\u{a0}' => {
                let len = self.take_while(|c| matches!(c, ' ' | '\t' | '\u{a0}'));
                self.push(WHITESPACE, len);
            }
            '\r' | '\n' => {
                let len = if rest.starts_with("\r\n") { 2 } else { 1 };
                let kind = if std::mem::take(&mut self.continued_comment) {
                    LINE_CONTINUATION
                } else {
                    NEWLINE
                };
                self.push(kind, len);
            }
            '\'' => {
                let len = line_len(rest);
                self.push(COMMENT, len);
            }
            '#' if self.line_start && !in_placeholder => {
                if let Some((len, closed)) = inline_java_len(rest) {
                    if !closed {
                        self.error(
                            self.pos,
                            "missing `#End If` for this `#If Java` block: the rest of the file \
                             is read as part of it",
                        );
                    }
                    self.push(INLINE_JAVA, len);
                } else {
                    let len = line_len(rest);
                    self.push(DIRECTIVE, len);
                }
            }
            '_' if self.continuation_len().is_some() => {
                let len = self.continuation_len().unwrap();
                self.push(LINE_CONTINUATION, len);
            }
            '_' if self.continuation_before_comment() => {
                // ` _ ' comment`: the original toolchain accepts it
                // (recorded). Without a space before the `'` it is open
                // (`docs/open-questions.md`): say so instead of reading `_`
                // as a name.
                let spaces = self.rest()[1..]
                    .bytes()
                    .take_while(|b| matches!(b, b' ' | b'\t'))
                    .count();
                if spaces == 0 {
                    self.error(
                        self.pos,
                        "a comment right after a line continuation `_` is not supported yet: \
                         put a space before the `'`",
                    );
                    self.push(ERROR_TOKEN, 1);
                } else {
                    self.push(LINE_CONTINUATION, 1 + spaces);
                    self.continued_comment = true;
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                let len = self.take_while(|c| c.is_alphanumeric() || c == '_');
                let kind = SyntaxKind::from_keyword(&rest[..len]).unwrap_or(IDENT);
                self.push(kind, len);
            }
            '0'..='9' => self.number(),
            '"' => self.string(),
            '$' if rest.starts_with("$\"") && self.smart_depth >= MAX_SMART_NESTING => {
                // Too deep: report it once, then lex the rest of this string
                // as plain code, which never recurses.
                if !self.reported_too_deep {
                    self.reported_too_deep = true;
                    self.error(
                        self.pos,
                        format!(
                            "smart strings nested too deeply (more than {MAX_SMART_NESTING} levels)"
                        ),
                    );
                }
                self.push(ERROR_TOKEN, 1);
            }
            '$' if rest.starts_with("$\"") => {
                self.smart_depth += 1;
                self.smart_string();
                self.smart_depth -= 1;
            }
            '}' if in_placeholder => return false,
            '<' if rest.starts_with("<>") => self.push(NEQ, 2),
            '<' if rest.starts_with("<=") => self.push(LT_EQ, 2),
            '>' if rest.starts_with(">=") => self.push(GT_EQ, 2),
            _ => {
                let kind = match c {
                    '(' => L_PAREN,
                    ')' => R_PAREN,
                    ',' => COMMA,
                    '.' => DOT,
                    ':' => COLON,
                    '=' => EQ,
                    '<' => LT,
                    '>' => GT,
                    '+' => PLUS,
                    '-' => MINUS,
                    '*' => STAR,
                    '/' => SLASH,
                    '&' => AMP,
                    _ => {
                        self.error(self.pos, format!("unexpected character '{c}'"));
                        ERROR_TOKEN
                    }
                };
                self.push(kind, c.len_utf8());
            }
        }
        true
    }

    /// Length of a line continuation starting at the current `_`: the `_`
    /// must follow whitespace and be followed only by spaces and a line break.
    /// ` _ ' comment`: a `_` after a space, followed only by spaces and a
    /// comment.
    fn continuation_before_comment(&self) -> bool {
        self.src[..self.pos].ends_with([' ', '\t'])
            && self.rest()[1..]
                .trim_start_matches([' ', '\t'])
                .starts_with('\'')
    }

    fn continuation_len(&self) -> Option<usize> {
        let after_space = self.src[..self.pos].ends_with([' ', '\t']);
        if !after_space {
            return None;
        }
        let rest = &self.rest()[1..];
        let spaces = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let tail = &rest[spaces..];
        let newline = if tail.starts_with("\r\n") {
            2
        } else if tail.starts_with(['\n', '\r']) {
            1
        } else {
            return None;
        };
        Some(1 + spaces + newline)
    }

    fn number(&mut self) {
        let rest = self.rest();
        let bytes = rest.as_bytes();
        if (rest.starts_with("0x") || rest.starts_with("0X"))
            && bytes.get(2).is_some_and(u8::is_ascii_hexdigit)
        {
            let digits = rest[2..].bytes().take_while(u8::is_ascii_hexdigit).count();
            self.push(INT, 2 + digits);
            return;
        }
        let digits = |from: usize| {
            bytes[from..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        };
        let mut len = digits(0);
        let mut kind = INT;
        if bytes.get(len) == Some(&b'.') && bytes.get(len + 1).is_some_and(u8::is_ascii_digit) {
            len += 1 + digits(len + 1);
            kind = REAL;
        }
        if matches!(bytes.get(len), Some(b'e' | b'E')) {
            let sign = usize::from(matches!(bytes.get(len + 1), Some(b'+' | b'-')));
            let exp = digits(len + 1 + sign);
            if exp > 0 {
                len += 1 + sign + exp;
                kind = REAL;
            }
        }
        self.push(kind, len);
    }

    /// `"..."`: no escapes, may span lines.
    fn string(&mut self) {
        let start = self.pos;
        match self.rest()[1..].find('"') {
            Some(i) => self.push(STRING, i + 2),
            None => {
                self.error(start, "unterminated string literal");
                let len = self.rest().len();
                self.push(STRING, len);
            }
        }
    }

    /// `$"...${expr}..."$`, with placeholders lexed as code.
    fn smart_string(&mut self) {
        let start = self.pos;
        self.push(SMART_STRING_START, 2);
        loop {
            let rest = self.rest();
            if rest.is_empty() {
                self.error(start, "unterminated smart string literal");
                return;
            }
            if rest.starts_with("\"$") {
                self.push(SMART_STRING_END, 2);
                return;
            }
            if let Some(len) = placeholder_start_len(rest) {
                let placeholder = self.pos;
                self.push(PLACEHOLDER_START, len);
                loop {
                    if self.at_end() {
                        self.error(placeholder, "unterminated smart string placeholder");
                        return;
                    }
                    if !self.code_token(true) {
                        self.push(PLACEHOLDER_END, 1);
                        break;
                    }
                }
                continue;
            }
            let len = smart_text_len(rest);
            self.push(SMART_STRING_TEXT, len);
        }
    }
}

/// Length up to (not including) the end of the line.
fn line_len(text: &str) -> usize {
    text.find(['\r', '\n']).unwrap_or(text.len())
}

/// The words of a directive line after its `#`, in lower case, up to a
/// trailing comment: `#End If ' native code` gives `end`, `if`.
fn directive_words(line: &str) -> Vec<String> {
    let line = line.split('\'').next().unwrap_or("");
    line.trim_start()
        .strip_prefix('#')
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect()
}

/// For a `#If Java` line: the length of the block up to the end of its
/// `#End If` line (not including the line break), and whether that line was
/// found; without it the block runs to the end of the text.
fn inline_java_len(text: &str) -> Option<(usize, bool)> {
    if directive_words(&text[..line_len(text)]) != ["if", "java"] {
        return None;
    }
    let mut start = 0;
    loop {
        let end = start + line_len(&text[start..]);
        if start > 0 && directive_words(&text[start..end]) == ["end", "if"] {
            return Some((end, true));
        }
        let rest = &text[end..];
        let newline = if rest.starts_with("\r\n") {
            2
        } else if rest.is_empty() {
            return Some((text.len(), false));
        } else {
            1
        };
        start = end + newline;
    }
}

/// Length of `$`, an optional formatter (`2`, `1.2`, `xml`…) and `{`.
fn placeholder_start_len(text: &str) -> Option<usize> {
    let after = text.strip_prefix('$')?;
    let fmt = after
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'.')
        .count();
    after[fmt..].starts_with('{').then_some(fmt + 2)
}

/// Length of literal text before the next `"$` or placeholder.
fn smart_text_len(text: &str) -> usize {
    let mut len = 0;
    for (i, c) in text.char_indices() {
        let at = &text[i..];
        if i > 0 && (at.starts_with("\"$") || placeholder_start_len(at).is_some()) {
            return i;
        }
        len = i + c.len_utf8();
    }
    len
}

/// Length of a B4J header ending with an `@EndOfDesignText@` line, if the
/// text (after any BOM) starts with one. Every line before it must look like
/// `Key=Value`. Project files and module files share this header.
pub fn header_len(text: &str) -> Option<usize> {
    let mut offset = 0;
    while offset < text.len() {
        let line = &text[offset..offset + line_len(&text[offset..])];
        let rest = &text[offset + line.len()..];
        let newline = if rest.starts_with("\r\n") {
            2
        } else {
            usize::from(!rest.is_empty())
        };
        if line == "@EndOfDesignText@" {
            return Some(offset + line.len() + newline);
        }
        let key = line.split_once('=').map(|(key, _)| key)?;
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return None;
        }
        offset += line.len() + newline;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(SyntaxKind, &str)> {
        let (tokens, errors) = lex(text);
        assert!(errors.is_empty(), "{errors:?}");
        let mut pos = 0;
        let out = tokens
            .iter()
            .map(|t| {
                let s = &text[pos..pos + t.len as usize];
                pos += t.len as usize;
                (t.kind, s)
            })
            .collect();
        assert_eq!(pos, text.len());
        out
    }

    fn significant(text: &str) -> Vec<(SyntaxKind, &str)> {
        kinds(text)
            .into_iter()
            .filter(|(k, _)| !k.is_trivia())
            .collect()
    }

    #[test]
    fn keywords_and_identifiers() {
        assert_eq!(
            significant("END sub Log_1"),
            [(END_KW, "END"), (SUB_KW, "sub"), (IDENT, "Log_1")]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            significant("12 0xFF 1.5 2.5e-3 1e5 3.x"),
            [
                (INT, "12"),
                (INT, "0xFF"),
                (REAL, "1.5"),
                (REAL, "2.5e-3"),
                (REAL, "1e5"),
                (INT, "3"),
                (DOT, "."),
                (IDENT, "x"),
            ]
        );
    }

    #[test]
    fn operators() {
        assert_eq!(
            significant("<> <= >= < > = & : ,"),
            [
                (NEQ, "<>"),
                (LT_EQ, "<="),
                (GT_EQ, ">="),
                (LT, "<"),
                (GT, ">"),
                (EQ, "="),
                (AMP, "&"),
                (COLON, ":"),
                (COMMA, ","),
            ]
        );
    }

    #[test]
    fn comments_newlines_and_continuations() {
        assert_eq!(
            kinds("a = 1 _\r\n + 2 ' c\nb_\n"),
            [
                (IDENT, "a"),
                (WHITESPACE, " "),
                (EQ, "="),
                (WHITESPACE, " "),
                (INT, "1"),
                (WHITESPACE, " "),
                (LINE_CONTINUATION, "_\r\n"),
                (WHITESPACE, " "),
                (PLUS, "+"),
                (WHITESPACE, " "),
                (INT, "2"),
                (WHITESPACE, " "),
                (COMMENT, "' c"),
                (NEWLINE, "\n"),
                (IDENT, "b_"),
                (NEWLINE, "\n"),
            ]
        );
    }

    #[test]
    fn underscore_without_space_is_not_a_continuation() {
        assert_eq!(
            significant("x)_\n"),
            [(IDENT, "x"), (R_PAREN, ")"), (IDENT, "_"), (NEWLINE, "\n")]
        );
    }

    #[test]
    fn strings_have_no_escapes_and_may_span_lines() {
        assert_eq!(
            significant("\"a\"\"b\" \"x\ny\""),
            [(STRING, "\"a\""), (STRING, "\"b\""), (STRING, "\"x\ny\"")]
        );
    }

    #[test]
    fn smart_strings() {
        assert_eq!(
            significant("$\"T: ${x} \"q\" $1.2{f(\"}\")}\"$"),
            [
                (SMART_STRING_START, "$\""),
                (SMART_STRING_TEXT, "T: "),
                (PLACEHOLDER_START, "${"),
                (IDENT, "x"),
                (PLACEHOLDER_END, "}"),
                (SMART_STRING_TEXT, " \"q\" "),
                (PLACEHOLDER_START, "$1.2{"),
                (IDENT, "f"),
                (L_PAREN, "("),
                (STRING, "\"}\""),
                (R_PAREN, ")"),
                (PLACEHOLDER_END, "}"),
                (SMART_STRING_END, "\"$"),
            ]
        );
    }

    #[test]
    fn smart_strings_with_non_ascii_text() {
        assert_eq!(
            significant("$\"café ${x} €$\"$"),
            [
                (SMART_STRING_START, "$\""),
                (SMART_STRING_TEXT, "café "),
                (PLACEHOLDER_START, "${"),
                (IDENT, "x"),
                (PLACEHOLDER_END, "}"),
                (SMART_STRING_TEXT, " €$"),
                (SMART_STRING_END, "\"$"),
            ]
        );
    }

    #[test]
    fn nested_smart_string_and_lone_dollar() {
        assert_eq!(
            significant("$\"a $ ${$\"b\"$}\"$"),
            [
                (SMART_STRING_START, "$\""),
                (SMART_STRING_TEXT, "a $ "),
                (PLACEHOLDER_START, "${"),
                (SMART_STRING_START, "$\""),
                (SMART_STRING_TEXT, "b"),
                (SMART_STRING_END, "\"$"),
                (PLACEHOLDER_END, "}"),
                (SMART_STRING_END, "\"$"),
            ]
        );
    }

    #[test]
    fn directives_only_at_line_start() {
        assert_eq!(
            significant("#Region X\n  #End Region\n"),
            [
                (DIRECTIVE, "#Region X"),
                (NEWLINE, "\n"),
                (DIRECTIVE, "#End Region"),
                (NEWLINE, "\n"),
            ]
        );
        let (_, errors) = lex("a #");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn inline_java_is_one_token() {
        let text = "#If Java\r\npublic static int x() { return '#'; }\r\n  #End If\r\nSub X\r\n";
        assert_eq!(
            significant(text)[..2],
            [
                (
                    INLINE_JAVA,
                    "#If Java\r\npublic static int x() { return '#'; }\r\n  #End If"
                ),
                (NEWLINE, "\r\n"),
            ]
        );
        assert!(lex(text).1.is_empty());
        // Any case; other `#If` lines stay directives.
        assert_eq!(significant("#IF JAVA\n#end if")[0].0, INLINE_JAVA);
        assert_eq!(significant("#If B4J\n#End If")[0], (DIRECTIVE, "#If B4J"));
        let (tokens, errors) = lex("#If Java\nint x;\n");
        assert_eq!(tokens.len(), 1);
        assert_eq!(errors.len(), 1, "missing `#End If`");
        // A trailing comment on either line does not hide it.
        let text = "#If Java ' native code\nint x;\n#End If 'end\nSub X\n";
        assert_eq!(
            significant(text)[0],
            (INLINE_JAVA, "#If Java ' native code\nint x;\n#End If 'end")
        );
        assert!(lex(text).1.is_empty());
        // The block ends at the first `#End If`: an `#If` inside it does not
        // nest.
        assert_eq!(
            significant("#If Java\n#If B4J\n#End If\n#End If\n")[..3],
            [
                (INLINE_JAVA, "#If Java\n#If B4J\n#End If"),
                (NEWLINE, "\n"),
                (DIRECTIVE, "#End If"),
            ]
        );
    }

    #[test]
    fn bom_and_header() {
        let text = "\u{feff}Type=StaticCode\r\nVersion=4.7\r\n@EndOfDesignText@\r\nSub X\r\n";
        let tokens = kinds(text);
        assert_eq!(tokens[0].0, BOM);
        assert_eq!(
            tokens[1],
            (
                HEADER,
                "Type=StaticCode\r\nVersion=4.7\r\n@EndOfDesignText@\r\n"
            )
        );
        assert_eq!(tokens[2], (SUB_KW, "Sub"));
    }

    #[test]
    fn no_header_without_end_marker() {
        assert_eq!(significant("a=1\n")[0], (IDENT, "a"));
        let (tokens, errors) = lex("Sub X\n@EndOfDesignText@\n");
        assert_eq!(tokens[0].kind, SUB_KW);
        assert_eq!(errors.len(), 2, "both stray @ are reported");
    }

    #[test]
    fn errors_are_reported_and_text_is_kept() {
        let (tokens, errors) = lex("a ? \"open");
        let total: u32 = tokens.iter().map(|t| t.len).sum();
        assert_eq!(total as usize, "a ? \"open".len());
        assert_eq!(errors.len(), 2);
    }
}
