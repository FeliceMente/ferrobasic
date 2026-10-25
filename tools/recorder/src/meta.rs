//! Rewriting a test's `.meta.toml` after a recording, line by line so that
//! its comments survive (`docs/compatibility-testing.md`).

/// How the recorded program ended, when that differs from what the
/// metadata said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    Normal,
    Exit(i32),
    Uncaught(String),
}

/// What goes in the `[recording]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    pub product: String,
    pub version: String,
    pub platform: String,
    pub java: String,
    /// The machine's own locale, and whether the output in it was the same
    /// as in the reference locale (`None`: it is the reference locale).
    pub machine_locale: String,
    pub machine_output_same: Option<bool>,
    /// A negative test the original toolchain refused to build.
    pub rejected: bool,
    pub date: String,
}

/// `meta` with `status = "recorded"` (when `recorded`), `ending` replacing
/// `exit_code` / `uncaught_error` (when given), no `[provisional]` table,
/// and `rec` as its `[recording]` table, replacing the commented template
/// or an earlier recording.
pub fn rewrite(meta: &str, recorded: bool, ending: Option<&Ending>, rec: &Recording) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut top = true;
    let mut lines = meta.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed == "[provisional]" || trimmed == "[recording]" {
            if out
                .last()
                .is_some_and(|l| l.starts_with("# Hand-derived output"))
            {
                out.pop();
            }
            // The table ends at a blank line, a comment or the next table.
            while lines.peek().is_some_and(|l| {
                let l = l.trim();
                !(l.is_empty() || l.starts_with('#') || l.starts_with('['))
            }) {
                lines.next();
            }
            top = false;
            continue;
        }
        if trimmed.starts_with("# Filled in by tools/recorder") {
            while lines.peek().is_some_and(|l| l.starts_with("# ")) {
                lines.next();
            }
            continue;
        }
        if trimmed.starts_with('[') {
            top = false;
        }
        match key(line).filter(|_| top) {
            Some("status") if recorded => {
                out.push(line.replacen("\"pending-recording\"", "\"recorded\"", 1));
                push_ending(&mut out, ending);
            }
            Some("status") => {
                out.push(line.to_string());
                push_ending(&mut out, ending);
            }
            Some("exit_code" | "uncaught_error") if ending.is_some() => {}
            _ => out.push(line.to_string()),
        }
    }
    // No runs of blank lines, and none at the end.
    out.dedup_by(|a, b| a.trim().is_empty() && b.trim().is_empty());
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    let mut text = out.join("\n");
    text.push_str("\n\n[recording]\n");
    let mut field = |k: &str, v: &str| text.push_str(&format!("{k} = {}\n", quote(v)));
    field("product", &rec.product);
    field("version", &rec.version);
    field("platform", &rec.platform);
    field("java", &rec.java);
    text.push_str(&format!(
        "locale = \"{}\"   # forced: the reference locale\n",
        crate::java::REFERENCE_LOCALE
    ));
    text.push_str(&format!(
        "machine_locale = {}\n",
        quote(&rec.machine_locale)
    ));
    match rec.machine_output_same {
        Some(true) => text.push_str("machine_locale_output = \"same\"\n"),
        Some(false) => text.push_str(&format!(
            "machine_locale_output = \"differs\"   # see the .{}.expected file\n",
            rec.machine_locale
        )),
        None => {}
    }
    if rec.rejected {
        text.push_str("rejected = true   # the original toolchain refused to build it\n");
    }
    text.push_str(&format!("date = {}\n", quote(&rec.date)));
    text
}

fn push_ending(out: &mut Vec<String>, ending: Option<&Ending>) {
    match ending {
        Some(Ending::Exit(code)) => out.push(format!("exit_code = {code}")),
        Some(Ending::Uncaught(kind)) => out.push(format!(
            "uncaught_error = {}   # kind of the error that stops the program",
            quote(kind)
        )),
        Some(Ending::Normal) | None => {}
    }
}

/// The key of a `key = value` line, if it is one.
fn key(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    Some(line.split_once('=')?.0.trim())
}

/// A TOML basic string.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const META: &str = "# Conformance test metadata. See docs/compatibility-testing.md.
status = \"pending-recording\"   # pending-recording | recorded | deviation | negative
exit_code = 3   # ExitApplication2(3)
level = 0
features = [\"exit-application2\"]

# Hand-derived output, used until a recording exists. See docs/compatibility-testing.md.
[provisional]
basis = \"source\"   # source | generated | docs | forum
refs = [
    \"docs/research/level0-semantics.md#keywords-maths-and-bit-source\",
]

# Filled in by tools/recorder when the expected output is recorded:
# [recording]
# product = \"B4J\"
# version = \"\"
# platform = \"\"
# date = \"\"
";

    fn recording() -> Recording {
        Recording {
            product: "B4J".into(),
            version: "10.30".into(),
            platform: "Windows 10 10.0".into(),
            java: "19.0.2 (Oracle Corporation)".into(),
            machine_locale: "it-IT".into(),
            machine_output_same: Some(true),
            rejected: false,
            date: "2026-10-25".into(),
        }
    }

    #[test]
    fn records_and_drops_the_provisional_table() {
        let text = rewrite(META, true, None, &recording());
        assert_eq!(
            text,
            "# Conformance test metadata. See docs/compatibility-testing.md.
status = \"recorded\"   # pending-recording | recorded | deviation | negative
exit_code = 3   # ExitApplication2(3)
level = 0
features = [\"exit-application2\"]

[recording]
product = \"B4J\"
version = \"10.30\"
platform = \"Windows 10 10.0\"
java = \"19.0.2 (Oracle Corporation)\"
locale = \"en-US\"   # forced: the reference locale
machine_locale = \"it-IT\"
machine_locale_output = \"same\"
date = \"2026-10-25\"
"
        );
    }

    #[test]
    fn replaces_the_ending_and_an_earlier_recording() {
        let first = rewrite(META, true, None, &recording());
        let mut rec = recording();
        rec.machine_output_same = Some(false);
        rec.version = "10.50".into();
        let text = rewrite(
            &first,
            true,
            Some(&Ending::Uncaught("java.lang.Error".into())),
            &rec,
        );
        assert!(!text.contains("exit_code"));
        assert!(text.contains(
            "status = \"recorded\"   # pending-recording | recorded | deviation | negative\n\
             uncaught_error = \"java.lang.Error\""
        ));
        assert_eq!(text.matches("[recording]").count(), 1);
        assert!(text.contains("version = \"10.50\"\n"));
        assert!(
            text.contains("machine_locale_output = \"differs\"   # see the .it-IT.expected file\n")
        );
        let normal = rewrite(&first, true, Some(&Ending::Normal), &recording());
        assert!(!normal.contains("exit_code") && !normal.contains("uncaught_error"));
    }

    #[test]
    fn keeps_other_statuses_and_marks_rejections() {
        let mut rec = recording();
        rec.rejected = true;
        rec.machine_output_same = None;
        let text = rewrite("status = \"negative\"\nlevel = 0\n", false, None, &rec);
        assert!(text.starts_with("status = \"negative\"\nlevel = 0\n\n[recording]\n"));
        assert!(text.contains("rejected = true"));
        assert!(!text.contains("machine_locale_output"));
    }

    #[test]
    fn quotes_strings() {
        assert_eq!(quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000A\"");
    }
}
