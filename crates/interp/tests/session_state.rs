use ferro_hir::SessionBindings;
use ferro_interp::SessionState;

fn program(body: &str, bindings: &SessionBindings) -> (ferro_mir::Program, SessionBindings) {
    let text =
        format!("Sub Process_Globals\n{body}\nEnd Sub\nSub AppStart(Args() As String)\nEnd Sub\n");
    let parse = ferro_syntax::parse(&text);
    assert!(parse.errors().is_empty());
    let (module, errors, bindings) = ferro_hir::check_session(&parse.syntax(), bindings);
    assert!(errors.is_empty(), "{errors:?}");
    (ferro_mir::lower(&module), bindings)
}

#[test]
fn storage_schema_and_validation_failures_do_not_destroy_session_values() {
    let mut state = SessionState::default();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (original, bindings) = program("Dim n As Int = 7", &SessionBindings::default());
    assert!(
        state
            .execute(&original, &mut out, &mut err)
            .outcome
            .error
            .is_none()
    );
    drop(original);
    let (wrong, _) = program("Dim n As String = \"changed\"", &SessionBindings::default());
    assert!(
        state
            .execute(&wrong, &mut out, &mut err)
            .outcome
            .error
            .is_some()
    );
    let (next, _) = program("Log(n)", &bindings);
    let mut malformed = next.clone();
    malformed.init = ferro_mir::FuncId(u32::MAX);
    assert!(
        state
            .execute(&malformed, &mut out, &mut err)
            .outcome
            .error
            .is_some()
    );
    assert!(
        state
            .execute(&next, &mut out, &mut err)
            .outcome
            .error
            .is_none()
    );
    assert_eq!(out, b"7\n");
}
