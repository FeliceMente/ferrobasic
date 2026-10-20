//! `Select` on a value that is not a number or a Char matches as
//! `BA.switchObjectToInt` does, raising the same Java exceptions.

/// Runs `body` in `AppStart`; returns stdout and the uncaught error's kind.
fn run(body: &str) -> (String, Option<String>) {
    let source = format!("Sub AppStart (Args() As String)\n{body}\nEnd Sub\n");
    let parse = ferro_syntax::parse(&source);
    assert!(parse.errors().is_empty(), "{:?}", parse.errors());
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "{diags:?}");
    let program = ferro_mir::lower(&module);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let outcome = ferro_interp::run(&program, &[], &mut stdout, &mut stderr);
    (
        String::from_utf8(stdout).unwrap(),
        outcome.error.map(|e| e.kind),
    )
}

fn select(scrutinee: &str, case: &str) -> (String, Option<String>) {
    run(&format!(
        "    Dim o As Object = {scrutinee}
    Select o
        Case {case}
            Log(\"matched\")
        Case Else
            Log(\"else\")
    End Select"
    ))
}

#[test]
fn a_number_matches_by_double_value() {
    assert_eq!(select("1", "1.0"), ("matched\n".into(), None));
    assert_eq!(select("2.5", "2"), ("else\n".into(), None));
    assert_eq!(select("\"1\"", "1"), ("else\n".into(), None));
}

#[test]
fn a_number_against_a_case_that_is_not_one_is_a_java_error() {
    let cast = Some("java.lang.ClassCastException".to_string());
    let null = Some("java.lang.NullPointerException".to_string());
    assert_eq!(select("1", "\"x\""), (String::new(), cast));
    assert_eq!(select("1", "Null"), (String::new(), null.clone()));
    assert_eq!(select("Null", "1"), (String::new(), null));
}
