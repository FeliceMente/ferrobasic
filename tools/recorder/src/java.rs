//! What the recorder asks of Java: its settings, and how a program run on it
//! is reported.

use std::collections::BTreeMap;

/// The settings `java -XshowSettings:properties -version` prints (on stderr):
/// lines `    key = value` under `Property settings:`.
pub fn properties(output: &str) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    let mut last: Option<String> = None;
    for line in output.lines() {
        if let Some((key, value)) = line.split_once(" = ") {
            let key = key.trim();
            if !key.is_empty() && !key.contains(' ') {
                props.insert(key.to_string(), value.trim().to_string());
                last = Some(key.to_string());
                continue;
            }
        }
        // A list value (`java.class.path`, …) continues on indented lines.
        if !line.starts_with("        ") {
            last = None;
        } else if let Some(key) = &last {
            let value = props.get_mut(key).expect("just inserted");
            value.push(' ');
            value.push_str(line.trim());
        }
    }
    props
}

/// The JVM and machine a recording comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// `19.0.2 (Oracle Corporation)`.
    pub java: String,
    /// `Windows 10 10.0`.
    pub platform: String,
    /// The default locale for formatting, as a tag: `it-IT`.
    pub locale: String,
}

impl Machine {
    pub fn from_properties(props: &BTreeMap<String, String>) -> Machine {
        let get = |k: &str| props.get(k).map(String::as_str).unwrap_or("");
        let java = match get("java.vendor") {
            "" => get("java.version").to_string(),
            vendor => format!("{} ({vendor})", get("java.version")),
        };
        let platform = format!("{} {}", get("os.name"), get("os.version"))
            .trim()
            .to_string();
        // Formatting follows the FORMAT category, which Windows can set
        // apart from the display language.
        let pick = |format: &str, plain: &str| match get(format) {
            "" => get(plain).to_string(),
            v => v.to_string(),
        };
        let language = pick("user.language.format", "user.language");
        let country = pick("user.country.format", "user.country");
        let locale = match (language.as_str(), country.as_str()) {
            ("", _) => "unknown".to_string(),
            (l, "") => l.to_string(),
            (l, c) => format!("{l}-{c}"),
        };
        Machine {
            java,
            platform,
            locale,
        }
    }
}

/// The reference locale every recording is made in.
pub const REFERENCE_LOCALE: &str = "en-US";

/// The arguments that run `jar`: in `en-US` when `forced` (every locale
/// category), else in the machine's own locale. Output is always UTF-8,
/// whatever the console's code page.
pub fn run_args(jar: &str, forced: bool) -> Vec<String> {
    let mut args = Vec::new();
    if forced {
        for category in ["", ".format", ".display"] {
            args.push(format!("-Duser.language{category}=en"));
            args.push(format!("-Duser.country{category}=US"));
        }
    }
    args.push("-Dstdout.encoding=UTF-8".into());
    args.push("-Dstderr.encoding=UTF-8".into());
    args.push("-jar".into());
    args.push(jar.into());
    args
}

/// The kind of the uncaught error in a program's stderr: the first line
/// that starts with a Java class name ending in `Exception` or `Error`
/// (after an optional `Exception in thread "main" `). `Caused by:` lines
/// and stack frames do not count.
pub fn uncaught_kind(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|line| {
        let line = line.trim();
        let line = line
            .strip_prefix("Exception in thread \"main\" ")
            .unwrap_or(line);
        let name = line.split([':', ' ', '\t']).next()?;
        let is_class = name.contains('.')
            && name.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
            });
        (is_class && (name.ends_with("Exception") || name.ends_with("Error")))
            .then(|| name.to_string())
    })
}

/// A program's stdout as text, with Windows line ends made `\n`.
pub fn normalize(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout).replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: &str = "Property settings:
    file.encoding = UTF-8
    java.class.path = 
    java.library.path = C:\\Java\\jdk-19.0.2\\bin
        C:\\WINDOWS\\Sun\\Java\\bin
    java.vendor = Oracle Corporation
    java.version = 19.0.2
    os.name = Windows 10
    os.version = 10.0
    user.country = IT
    user.language = it

openjdk version \"19.0.2\" 2023-01-17
";

    #[test]
    fn reads_the_machine_from_the_settings() {
        let props = properties(SETTINGS);
        assert_eq!(
            props["java.library.path"],
            "C:\\Java\\jdk-19.0.2\\bin C:\\WINDOWS\\Sun\\Java\\bin"
        );
        assert_eq!(props["java.class.path"], "");
        assert_eq!(
            Machine::from_properties(&props),
            Machine {
                java: "19.0.2 (Oracle Corporation)".into(),
                platform: "Windows 10 10.0".into(),
                locale: "it-IT".into(),
            }
        );
    }

    #[test]
    fn the_format_locale_wins() {
        let props = properties(
            "    user.language = en\n    user.country = US\n    user.language.format = it\n    user.country.format = IT\n",
        );
        assert_eq!(Machine::from_properties(&props).locale, "it-IT");
        assert_eq!(Machine::from_properties(&BTreeMap::new()).locale, "unknown");
    }

    #[test]
    fn forcing_the_locale_sets_every_category() {
        let args = run_args("Objects\\t.jar", true);
        assert!(args.contains(&"-Duser.language.format=en".to_string()));
        assert!(args.contains(&"-Duser.country=US".to_string()));
        assert_eq!(args[args.len() - 2..], ["-jar", "Objects\\t.jar"]);
        let args = run_args("t.jar", false);
        assert!(!args.iter().any(|a| a.starts_with("-Duser.")));
        assert!(args.contains(&"-Dstdout.encoding=UTF-8".to_string()));
    }

    #[test]
    fn finds_the_uncaught_error_kind() {
        let trace = "main._appstart (java line: 52)\r\njava.lang.RuntimeException: Object should first be initialized (Map).\r\n\tat anywheresoftware.b4a.AbsObjectWrapper.getObject(AbsObjectWrapper.java:50)\r\nCaused by: java.lang.NullPointerException\r\n";
        assert_eq!(
            uncaught_kind(trace).as_deref(),
            Some("java.lang.RuntimeException")
        );
        assert_eq!(
            uncaught_kind("Exception in thread \"main\" java.lang.StackOverflowError\n").as_deref(),
            Some("java.lang.StackOverflowError")
        );
        assert_eq!(
            uncaught_kind("Program started.\nAn Exception: not a class\n"),
            None
        );
        assert_eq!(uncaught_kind(""), None);
    }

    #[test]
    fn normalizes_line_ends_only() {
        assert_eq!(normalize(b"a\r\nb\rc\n"), "a\nb\rc\n");
    }
}
