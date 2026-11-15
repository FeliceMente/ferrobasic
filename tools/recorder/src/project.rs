//! A conformance test, wrapped in the smallest console project the original
//! toolchain builds (`docs/spec/project-files.md`).

/// The project file for a test's code: a console header, then the main
/// module. The code is kept byte for byte (an existing module header is
/// dropped), after the project attributes a new console project starts
/// with; `MergeLibraries` makes the jar run on its own.
pub fn project_file(code: &str) -> String {
    let code = match code.split_once("@EndOfDesignText@") {
        Some((_, rest)) => rest
            .strip_prefix("\r\n")
            .or(rest.strip_prefix('\n'))
            .unwrap_or(rest),
        None => code,
    };
    let nl = if code.contains("\r\n") { "\r\n" } else { "\n" };
    let header = [
        "AppType=StandardJava",
        "Build1=Default,b4j.example",
        "Group=Default Group",
        "Library1=jcore",
        "NumberOfFiles=0",
        "NumberOfLibraries=1",
        "NumberOfModules=0",
        "Version=10",
        "@EndOfDesignText@",
        "#Region Project Attributes",
        "\t#CommandLineArgs:",
        "\t#MergeLibraries: True",
        "#End Region",
        "",
    ];
    let mut out = header.join(nl);
    out.push_str(nl);
    out.push_str(code);
    out
}

/// The line of the test's code `code` that is line `line` of the project
/// file [`project_file`] makes from it, counted as the builder counts: from
/// the first line after the project's `@EndOfDesignText@`. `None` when that
/// line is not the test's code, or does not hold `expected` (the code the
/// builder shows for it, trimmed): then the count is not the one assumed.
pub fn test_line(code: &str, line: usize, expected: &str) -> Option<usize> {
    let project = project_file(code);
    let after_header = project
        .lines()
        .skip_while(|l| *l != "@EndOfDesignText@")
        .skip(1);
    // The project's own lines after the header, before the test's code.
    let own = after_header.clone().take_while(|l| !l.is_empty()).count() + 1;
    let shown = after_header.clone().nth(line.checked_sub(1)?)?;
    if line <= own || shown.trim() != expected.trim() {
        return None;
    }
    // A module header the project dropped still counts in the test's file.
    let dropped = match code.split_once("@EndOfDesignText@") {
        Some((head, _)) => head.lines().count() + 1,
        None => 0,
    };
    Some(line - own + dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_builder_line_to_the_test() {
        // Recording run 21: the builder's line 15 is line 10 of the test.
        let code = "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n\n\n\n    Dim o As Object = 5\n    Try\n        For Each v In o\n";
        assert_eq!(test_line(code, 15, "For Each v In o"), Some(10));
        // Not the code the builder showed, or not the test's code.
        assert_eq!(test_line(code, 15, "Log(v)"), None);
        assert_eq!(test_line(code, 2, "#CommandLineArgs:"), None);
        assert_eq!(test_line(code, 99, ""), None);
        assert_eq!(test_line(code, 0, ""), None);
        // A dropped module header.
        let code =
            "Type=StaticCode\r\nVersion=10\r\n@EndOfDesignText@\r\nSub AppStart\r\nLog(x)\r\n";
        assert_eq!(test_line(code, 7, "Log(x)"), Some(5));
    }

    #[test]
    fn wraps_the_code_unchanged() {
        let code = "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n    Log(\"a\")\nEnd Sub\n";
        let file = project_file(code);
        assert!(file.starts_with("AppType=StandardJava\nBuild1=Default,b4j.example\n"));
        assert!(file.contains("\n@EndOfDesignText@\n#Region Project Attributes\n"));
        assert!(file.ends_with(&format!("#End Region\n\n{code}")));
    }

    #[test]
    fn drops_a_module_header_and_keeps_crlf() {
        let file =
            project_file("Type=StaticCode\r\n@EndOfDesignText@\r\nSub AppStart\r\nEnd Sub\r\n");
        assert!(!file.contains("Type=StaticCode"));
        assert!(file.starts_with("AppType=StandardJava\r\n"));
        assert!(file.ends_with("#End Region\r\n\r\nSub AppStart\r\nEnd Sub\r\n"));
        assert_eq!(file.matches("@EndOfDesignText@").count(), 1);
    }
}
