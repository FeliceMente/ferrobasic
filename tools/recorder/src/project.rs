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

#[cfg(test)]
mod tests {
    use super::*;

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
