Sub Process_Globals
    Type A (N As Object)
    Type B (N As Object)
End Sub

Sub AppStart (Args() As String)
    ' Recording run 25: does SortType's innermost cause keep Field.get's text?
    ' The exception kind is recorded in list_sort_type_errors (A, B).
    ' JDK reflection text gives kind=true, detail=true, field=true.
    ' A bare cause gives kind=true, detail=false, field=false, as FerroBASIC
    ' does meanwhile. A retained RuntimeException wrapper gives kind=false;
    ' another detail gives detail=true, field=false. No error prints
    ' "no exception". No generated class name or whole message is logged.
    ' See docs/open-questions.md, JDK-generated exception messages.
    Dim First As A
    First.Initialize
    First.N = 2
    Dim Second As B
    Second.Initialize
    Second.N = 1
    Dim Items As List
    Items.Initialize
    Items.Add(First)
    Items.Add(Second)
    Try
        Items.SortType("N", True)
        Log("no exception")
    Catch
        Log("kind=" & LastException.Message.StartsWith("java.lang.IllegalArgumentException"))
        Log("detail=" & LastException.Message.Contains(": "))
        Log("field=" & LastException.Message.Contains("field"))
    End Try
End Sub
