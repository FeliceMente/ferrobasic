use std::io::Write;
use std::process::{Command, Stdio};
const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

#[test]
fn repl_process_accepts_piped_input_without_prompts() {
    let mut child = Command::new(FERRO)
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Dim n As Int = 2\nn + 3\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"5\n");
    assert!(output.stderr.is_empty());
    let output = Command::new(FERRO)
        .args(["repl", "file.bas"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let output = Command::new(FERRO).arg("help").output().unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.split("Planned").next().unwrap().contains("repl"));
    assert!(!help.split("Planned").nth(1).unwrap().contains("repl"));
}
