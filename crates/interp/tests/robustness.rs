//! Valid programs with very deep or very large data must run, or stop with
//! the Java error B4J raises, never crash the process. These run on a test
//! thread (2 MB stack), smaller than `ferro`'s own.

use ferro_interp::Outcome;

/// Parses, checks, lowers and runs `source`; returns stdout and the outcome.
fn run(source: &str) -> (String, Outcome) {
    let parse = ferro_syntax::parse(source);
    assert!(parse.errors().is_empty(), "{:?}", parse.errors());
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "{diags:?}");
    let program = ferro_mir::lower(&module);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let outcome = ferro_interp::run(&program, &[], &mut stdout, &mut stderr);
    (String::from_utf8(stdout).unwrap(), outcome)
}

fn app_start(body: &str) -> String {
    format!("Sub AppStart (Args() As String)\n{body}\nEnd Sub\n")
}

/// Runs `body` in `AppStart` and expects it to print `expected` and end
/// normally.
fn runs(body: &str, expected: &str) {
    let (stdout, outcome) = run(&app_start(body));
    assert_eq!(outcome.error, None);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(stdout, expected);
}

/// Runs `source` and expects it to print `expected`, then stop on an
/// uncaught error of `kind`.
fn stops(source: &str, expected: &str, kind: &str) {
    let (stdout, outcome) = run(source);
    assert_eq!(outcome.exit_code, 1);
    assert_eq!(outcome.error.as_ref().map(|e| e.kind.as_str()), Some(kind));
    assert_eq!(stdout, expected);
}

/// Freeing a structure used to recurse once per level, and crashed at about
/// 30,000 levels on this stack.
#[test]
fn deep_structures_are_freed_without_recursion() {
    let linked = "Sub Process_Globals
    Type Node (Value As Int, NextNode As Node)
End Sub

Sub AppStart (Args() As String)
    Dim Head As Node
    For i = 1 To 100000
        Dim n As Node
        n.Initialize
        n.Value = i
        n.NextNode = Head
        Head = n
    Next
    Log(Head.Value)
End Sub
";
    let (stdout, outcome) = run(linked);
    assert_eq!(outcome.error, None);
    assert_eq!(stdout, "100000\n");

    runs(
        "    Dim Head As List
    Head.Initialize
    For i = 1 To 100000
        Dim l As List
        l.Initialize
        l.Add(Head)
        Head = l
    Next
    Log(Head.Size)",
        "1\n",
    );
    runs(
        "    Dim Head As Map
    Head.Initialize
    For i = 1 To 100000
        Dim m As Map
        m.Initialize
        m.Put(\"next\", Head)
        m.Put(Head, i)
        Head = m
    Next
    Log(Head.Size)",
        "2\n",
    );
    runs(
        "    Dim Head As Object
    For i = 1 To 100000
        Dim a(1) As Object
        a(0) = Head
        Head = a
    Next
    Log(\"done\")",
        "done\n",
    );
}

/// A `SubList` of a `SubList` is a view of the original list, not a chain
/// of views that every access walks.
#[test]
fn sub_lists_of_sub_lists_do_not_chain() {
    runs(
        "    Dim l As List
    l.Initialize
    l.AddAll(Array As Int(1, 2, 3))
    Dim v As List = l
    For i = 1 To 100000
        v = v.SubList(0, 3)
    Next
    Log(v.Get(2))
    Dim w As List = v.SubList(1, 3)
    Log(w.SubList(1, 2).Get(0))
    l.Set(2, 9)
    Log(w)
    l.Add(4)
    Try
        Log(w.Get(0))
    Catch
        Log(LastException.Message)
    End Try",
        "3\n3\n(UnmodifiableRandomAccessList) [2, 9]\njava.util.ConcurrentModificationException\n",
    );
}

/// Printing recurses in Java too: past 1000 levels it raises
/// `StackOverflowError`, which `Try` does not catch.
#[test]
fn printing_deep_structures_overflows_like_java() {
    let nested = |depth: usize, tail: &str| {
        app_start(&format!(
            "    Dim Head As List
    Head.Initialize
    For i = 2 To {depth}
        Dim l As List
        l.Initialize
        l.Add(Head)
        Head = l
    Next
    Log(\"built\")
{tail}"
        ))
    };
    let (stdout, outcome) = run(&nested(1000, "    Log(Head)"));
    assert_eq!(outcome.error, None);
    let expected = format!(
        "built\n(ArrayList) {}{}\n",
        "[".repeat(1000),
        "]".repeat(1000)
    );
    assert_eq!(stdout, expected);

    stops(
        &nested(1001, "    Log(Head)"),
        "built\n",
        "java.lang.StackOverflowError",
    );
    let caught = "    Try
        Dim s As String = Head
        Log(s.Length)
    Catch
        Log(\"caught\")
    End Try";
    stops(
        &nested(100000, caught),
        "built\n",
        "java.lang.StackOverflowError",
    );
}

/// A record whose List holds the record itself: Java's `toString` never
/// ends, because the List prints through its wrapper, which starts over.
#[test]
fn a_record_printing_itself_through_a_list_overflows() {
    stops(
        "Sub Process_Globals
    Type Box (Items As List)
End Sub

Sub AppStart (Args() As String)
    Dim b As Box
    b.Initialize
    b.Items.Initialize
    b.Items.Add(b)
    Log(\"built\")
    Log(b)
End Sub
",
        "built\n",
        "java.lang.StackOverflowError",
    );
}

/// Maps whose keys keep changing stay as small as their live entries, and
/// keep their order.
#[test]
fn map_churn_keeps_the_order() {
    runs(
        "    Dim m As Map
    m.Initialize
    m.Put(\"first\", 0)
    For i = 1 To 100000
        m.Put(i, i)
        m.Remove(i - 1)
    Next
    m.Put(\"last\", 0)
    For Each k As Object In m.Keys
        Log(k)
    Next
    Log(m.GetKeyAt(1))",
        "first\n100000\nlast\n100000\n",
    );
}
