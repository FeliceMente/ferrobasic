//! Valid programs with very deep or very large data must run, or stop with
//! the Java error B4J raises, never crash the process. These run on a test
//! thread (2 MB stack), smaller than `ferro`'s own.

mod common;

use common::{app_start, run};

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

/// Printing recurses in Java too: past 7000 levels (FerroBASIC's limit,
/// deviation D2) it raises
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
    let (stdout, outcome) = run(&nested(7000, "    Log(Head)"));
    assert_eq!(outcome.error, None);
    let expected = format!(
        "built\n(ArrayList) {}{}\n",
        "[".repeat(7000),
        "]".repeat(7000)
    );
    assert_eq!(stdout, expected);

    stops(
        &nested(7001, "    Log(Head)"),
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

/// Comparing recurses in Java: past 3000 levels (FerroBASIC's limit,
/// deviation D2) it raises `StackOverflowError`.
#[test]
fn comparing_deep_structures_overflows_like_java() {
    let nested = |depth: usize| {
        app_start(&format!(
            "    Dim A, B As List
    A.Initialize
    B.Initialize
    For i = 2 To {depth}
        Dim NextA, NextB As List
        NextA.Initialize
        NextA.Add(A)
        A = NextA
        NextB.Initialize
        NextB.Add(B)
        B = NextB
    Next
    Log(\"built\")
    Dim OA As Object = A
    Dim OB As Object = B
    Log(OA = OB)"
        ))
    };
    let (stdout, outcome) = run(&nested(3000));
    assert_eq!(outcome.error, None);
    assert_eq!(stdout, "built\ntrue\n");
    stops(&nested(3001), "built\n", "java.lang.StackOverflowError");
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

/// An empty array of Lists, records or arrays has no elements, even though
/// a non-empty one creates each element.
#[test]
fn empty_reference_arrays_are_empty() {
    let (stdout, outcome) = run("Sub Process_Globals
    Type Point (X As Int)
End Sub

Sub AppStart (Args() As String)
    Dim Lists(0) As List
    Dim Points(0) As Point
    Dim Arrays(0) As Int()
    For Each l As List In Lists
        Log(\"list\")
    Next
    For Each p As Point In Points
        Log(\"point\")
    Next
    Dim AsList As List
    AsList.Initialize2(Lists)
    Log(Lists.Length & \" \" & Points.Length & \" \" & Arrays.Length & \" \" & AsList.Size)
    Dim Two(2) As List
    Log(Two.Length & \" \" & Two(1).IsInitialized)
End Sub
");
    assert_eq!(outcome.error, None);
    assert_eq!(stdout, "0 0 0 0\n2 false\n");
}

/// An array of rank 2 is an array of rows, as in Java: each element of
/// every row gets its own List.
#[test]
fn multi_dimensional_arrays_have_an_element_per_cell() {
    runs(
        "    Dim Grid(2, 2) As List
    Grid(0, 0).Initialize
    Log(Grid(0, 1).IsInitialized & \" \" & Grid(1, 0).IsInitialized)
    Dim Empty(3, 0) As Int
    Log(Empty.Length)",
        "false false\n3\n",
    );
}
