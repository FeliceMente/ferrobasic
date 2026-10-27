//! Constructs that Level 0 does not support are reported precisely, one
//! error each, by `ferro check` (`docs/language-support.md`, #32).

use std::fs;
use std::path::Path;
use std::process::Command;

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

const PROGRAM: &str = "\
Sub Process_Globals
\tPrivate Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
\tDim o As Object = Size
\tIf o Is Int Then Log(o)
\tWait For (o) Complete (Success As Boolean)
End Sub

#If Java
public static int twice(int x) { return 2 * x; }
#End If
";

const EXPECTED: &str = "\
later.bas:2:10: error: `Const` declarations are not supported yet: planned for Level 0
  \tPrivate Const Size As Int = 10
  \t        ^
later.bas:7:5: error: type tests (`Is`) are not supported yet
  \tIf o Is Int Then Log(o)
  \t   ^
later.bas:8:2: error: `Wait For` is not supported at Level 0: it needs resumable subs (Level 2)
  \tWait For (o) Complete (Success As Boolean)
  \t^
later.bas:11:1: error: inline Java (`#If Java`) is not supported at Level 0: it needs Java interop (Level 3)
  #If Java
  ^
4 error(s)
";

#[test]
fn later_constructs_are_reported_one_error_each() {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("unsupported-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("later.bas"), PROGRAM).unwrap();
    let out = Command::new(FERRO)
        .args(["check", "later.bas"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&out.stderr), EXPECTED);
}
