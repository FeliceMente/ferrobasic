//! Members on a read-only `SubList` view check what jCore's `List.java`
//! checks, in its order, before the view refuses the change (#21). The
//! conformance test `level0/list_stale_sublist` covers the members whose
//! output is settled; these cover the exception kinds it cannot print.

/// Runs a program with `body` in `AppStart` and a record type `Item`;
/// returns the uncaught error's kind.
fn error_of(body: &str) -> Option<String> {
    let source = format!(
        "Sub Process_Globals\n    Type Item (N As Int)\nEnd Sub\n\
         Sub AppStart (Args() As String)\n{body}\nEnd Sub\n"
    );
    let parse = ferro_syntax::parse(&source);
    assert!(parse.errors().is_empty(), "{:?}", parse.errors());
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "{diags:?}");
    let program = ferro_mir::lower(&module);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    ferro_interp::run(&program, &[], &mut stdout, &mut stderr)
        .error
        .map(|e| e.kind)
}

const LIST: &str = "    Dim Items As List
    Items.Initialize";

#[test]
fn sort_type_on_a_view_looks_at_its_first_item_first() {
    // A `Null` first item: `o1.getClass()` raises before the sort.
    let body = format!(
        "{LIST}\n    Items.Add(Null)\n    Dim T As Item\n    T.Initialize\n    Items.Add(T)\n    \
         Dim V As List = Items.SubList(0, 2)\n    V.SortType(\"N\", True)"
    );
    assert_eq!(
        error_of(&body).as_deref(),
        Some("java.lang.NullPointerException")
    );
    // A first item that is not a record has no such field.
    let body = format!(
        "{LIST}\n    Items.AddAll(Array As Int(2, 1))\n    \
         Dim V As List = Items.SubList(0, 2)\n    V.SortTypeCaseInsensitive(\"N\", True)"
    );
    assert_eq!(
        error_of(&body).as_deref(),
        Some("java.lang.NoSuchFieldException")
    );
    // An empty view returns before anything else.
    let body = format!(
        "{LIST}\n    Items.Add(1)\n    Dim V As List = Items.SubList(1, 1)\n    \
         V.SortType(\"Missing\", True)"
    );
    assert_eq!(error_of(&body), None);
}

#[test]
fn sort_type_on_a_stale_view_reads_its_size_first() {
    for member in ["SortType", "SortTypeCaseInsensitive"] {
        let body = format!(
            "{LIST}\n    Items.AddAll(Array As Int(2, 1))\n    \
             Dim V As List = Items.SubList(0, 0)\n    Items.Add(3)\n    \
             V.{member}(\"N\", True)"
        );
        assert_eq!(
            error_of(&body).as_deref(),
            Some("java.util.ConcurrentModificationException"),
            "{member}"
        );
    }
}
