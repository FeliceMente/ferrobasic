Sub Process_Globals
    ' A module variable read in Process_Globals before its Dim there: B4J
    ' refuses a read before the declaration in a Sub
    ' (for_each_untyped_read_before). Here too? FerroBASIC accepts it, and
    ' the read gives Null (docs/open-questions.md).
    Log(n)
    Dim n As Object
End Sub

Sub AppStart (Args() As String)
    Log(GetType(n))
End Sub
