//! `SortType` reports the first problem in list order, where Java's
//! `TimSort` reports the first pair it compares (`docs/deviations.md`, D4;
//! conformance test `level0/sort_type_order_d4`).

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
