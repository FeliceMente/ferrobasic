//! What the original toolchain's builder says when it refuses a program: the
//! error and the line it names, read from its output.
//!
//! Two forms are known. The toolchain's own checks print
//!
//! ```text
//! Error parsing program.
//! Error description: Sub process_globals is missing.
//! Error occurred on line: 7 (Main)
//! Log(x)
//! ```
//!
//! and an error of the Java compiler on the generated code prints the line
//! first (recording run 21, `docs/research/recording-run-21.md`):
//!
//! ```text
//! Error: B4J line: 15
//! For Each v In o
//! src\b4j\example\main.java:46: error: incompatible types: ...
//! ```
//!
//! Output in neither form (the builder did not start, a library is missing,
//! ...) is not read as a refusal of the program's code.

/// The first error in the builder's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The error: the toolchain's description, or `javac: ` and the Java
    /// compiler's message.
    pub message: String,
    /// The line the builder names, counted as the builder counts: from the
    /// first line after the project file's `@EndOfDesignText@`.
    pub line: Option<usize>,
    /// The code the builder shows for that line, trimmed.
    pub code: Option<String>,
}

/// The refusal in the builder's output `log`, if it has one in a known form.
pub fn parse(log: &str) -> Option<Refusal> {
    let lines: Vec<&str> = log.lines().map(str::trim).collect();
    if let Some(i) = lines.iter().position(|l| l.contains("Error description:")) {
        let message = after(lines[i], "Error description:")?;
        let (line, code) = match lines
            .iter()
            .position(|l| l.starts_with("Error occurred on line:"))
        {
            Some(j) => (
                number(after(lines[j], "Error occurred on line:").as_deref()),
                code(lines.get(j + 1)),
            ),
            None => (None, None),
        };
        return Some(Refusal {
            message,
            line,
            code,
        });
    }
    let marker = lines.iter().position(|l| l.contains("B4J line:"));
    let from = marker.unwrap_or(0);
    let javac = lines[from..]
        .iter()
        .find_map(|l| l.split_once(": error: ").map(|(_, m)| m.trim()))
        .filter(|m| !m.is_empty())?;
    let (line, code) = match marker {
        Some(i) => (
            number(after(lines[i], "B4J line:").as_deref()),
            code(lines.get(i + 1)).filter(|c| !c.contains(": error: ")),
        ),
        None => (None, None),
    };
    Some(Refusal {
        message: format!("javac: {javac}"),
        line,
        code,
    })
}

/// What follows `marker` in `line`, trimmed, if anything does.
fn after(line: &str, marker: &str) -> Option<String> {
    let (_, rest) = line.split_once(marker)?;
    let rest = rest.trim();
    (!rest.is_empty()).then(|| rest.to_string())
}

/// The number `text` starts with: `7` in `7 (Main)`.
fn number(text: Option<&str>) -> Option<usize> {
    let text = text?;
    let digits = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    text[..digits].parse().ok()
}

/// The code line the builder shows after the line number.
fn code(line: Option<&&str>) -> Option<String> {
    let line = line?;
    (!line.is_empty() && !line.starts_with("Word:")).then(|| line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The builder output of recording run 21, with its Windows line ends.
    const JAVAC: &str = "B4JBuilder v10.70\r\nParsing code.\r\nCompiling generated Java code.\r\n\
        System.Exception: Error: B4J line: 15\r\nFor Each v In o\r\n\
        src\\b4j\\example\\main.java:46: error: incompatible types: Object cannot be converted to IterableList\r\n\
        final anywheresoftware.b4a.BA.IterableList group3 = _o;\r\n1 error\r\n\r\n   in h8.a(a A_0)\r\n\
        Error: B4J line: 15\r\nFor Each v In o\r\n\
        src\\b4j\\example\\main.java:46: error: incompatible types: Object cannot be converted to IterableList\r\n";

    #[test]
    fn reads_a_java_compiler_error() {
        assert_eq!(
            parse(JAVAC),
            Some(Refusal {
                message: "javac: incompatible types: Object cannot be converted to IterableList"
                    .into(),
                line: Some(15),
                code: Some("For Each v In o".into()),
            })
        );
    }

    #[test]
    fn reads_a_toolchain_error() {
        let log = "Parsing code.    (0.00s)\nError parsing program.\n\
            Error description: Undeclared variable 'x' is used before it was assigned any value.\n\
            Error occurred on line: 7 (Main)\nLog(x)\nWord: x\n";
        assert_eq!(
            parse(log),
            Some(Refusal {
                message: "Undeclared variable 'x' is used before it was assigned any value.".into(),
                line: Some(7),
                code: Some("Log(x)".into()),
            })
        );
        // Without the line, the description alone.
        assert_eq!(
            parse("Error parsing program. Error description: Sub process_globals is missing.\n"),
            Some(Refusal {
                message: "Sub process_globals is missing.".into(),
                line: None,
                code: None,
            })
        );
    }

    #[test]
    fn ignores_output_without_a_known_error() {
        assert_eq!(parse("Error compiling program.\r\n"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("Error description:\n"), None);
    }
}
