//! `SortType` and `SortTypeCaseInsensitive`: which errors are wrapped, the
//! `IsInitialized` field, and the cause reported in a list with several
//! problems.

/// FerroBASIC reports the first problem in list order, where Java's
/// `TimSort` reports the first pair it compares (`docs/deviations.md`, D4;
/// conformance test `level0/sort_type_order_d4`).

#[test]
fn sort_type_reports_the_first_problem_in_list_order() {
    let source = "Sub Process_Globals\n    Type A (N As Object)\n    Type B (N As Object)\nEnd Sub\n\
        Sub AppStart (Args() As String)\n    Dim X, Y As A\n    X.Initialize\n    X.N = 1\n    \
        Y.Initialize\n    Y.N = \"x\"\n    Dim Z As B\n    Z.Initialize\n    Dim L As List\n    \
        L.Initialize\n    L.Add(X)\n    L.Add(Y)\n    L.Add(Z)\n    L.SortType(\"N\", True)\nEnd Sub\n";
    let parse = ferro_syntax::parse(source);
    assert!(parse.errors().is_empty(), "{:?}", parse.errors());
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "{diags:?}");
    let program = ferro_mir::lower(&module);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let error = ferro_interp::run(&program, &[], &mut stdout, &mut stderr)
        .error
        .unwrap();
    // Java: `RuntimeException` caused by `ClassCastException` (Y against X).
    assert_eq!(error.kind, "java.lang.RuntimeException");
    assert_eq!(error.message, "java.lang.IllegalArgumentException");
}

/// Runs `body` in `AppStart`, with `Type A (N As Object)`; returns the
/// uncaught error's kind and message, and what the program logged.
fn run(body: &str) -> (Option<(String, String)>, String) {
    let source = format!(
        "Sub Process_Globals\n    Type A (N As Object)\nEnd Sub\n\
         Sub AppStart (Args() As String)\n{body}\nEnd Sub\n"
    );
    let parse = ferro_syntax::parse(&source);
    assert!(parse.errors().is_empty(), "{:?}", parse.errors());
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "{diags:?}");
    let program = ferro_mir::lower(&module);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let error = ferro_interp::run(&program, &[], &mut stdout, &mut stderr)
        .error
        .map(|e| (e.kind.to_string(), e.message));
    (error, String::from_utf8(stdout).unwrap())
}

#[test]
fn sort_type_wraps_exceptions_but_not_errors() {
    // The comparator catches `Exception`: printing a field nested too deeply
    // raises `StackOverflowError`, which is not wrapped.
    let (error, _) = run("    Dim Deep As List\n    Deep.Initialize\n    \
         For I = 1 To 2000\n        Dim Outer As List\n        Outer.Initialize\n        \
         Outer.Add(Deep)\n        Deep = Outer\n    Next\n    \
         Dim X, Y As A\n    X.Initialize\n    X.N = Deep\n    Y.Initialize\n    Y.N = Deep\n    \
         Dim L As List\n    L.Initialize\n    L.Add(X)\n    L.Add(Y)\n    \
         L.SortTypeCaseInsensitive(\"N\", True)");
    assert_eq!(error.unwrap().0, "java.lang.StackOverflowError");
}

#[test]
fn sort_type_case_insensitive_wraps_the_field_text_error() {
    // `String.valueOf` of a stale view raises inside the comparator.
    let (error, _) = run("    Dim P As List
    P.Initialize
    P.Add(1)
             Dim X, Y As A
    X.Initialize
    X.N = P.SubList(0, 1)
    P.Add(2)
             Y.Initialize
    Y.N = 1
             Dim L As List
    L.Initialize
    L.Add(X)
    L.Add(Y)
             L.SortTypeCaseInsensitive(\"N\", True)");
    assert_eq!(
        error,
        Some((
            "java.lang.RuntimeException".to_string(),
            "java.util.ConcurrentModificationException".to_string()
        ))
    );
}

#[test]
fn sort_type_sorts_by_is_initialized() {
    // Every `Type` class has a `boolean IsInitialized` field (false first).
    let (error, out) = run("    Dim U, I As A\n    I.Initialize\n    \
         Dim L As List\n    L.Initialize\n    L.Add(I)\n    L.Add(U)\n    \
         L.SortType(\"IsInitialized\", True)\n    Log(L.Get(0) = U)\n    \
         L.SortTypeCaseInsensitive(\"IsInitialized\", False)\n    Log(L.Get(0) = I)");
    assert_eq!(error, None);
    assert_eq!(out, "true\ntrue\n");
}
