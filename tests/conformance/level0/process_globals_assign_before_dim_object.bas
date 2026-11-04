Sub Process_Globals
    ' A module variable assigned in Process_Globals before its Dim there as
    ' an Object: in a Sub, an assignment before the declaration declares a
    ' String (for_each_untyped_assigned_before). Here, one variable, which
    ' the Dim then resets to a new Object? FerroBASIC accepts it
    ' (docs/open-questions.md).
    n = 5
    Dim n As Object
End Sub

Sub AppStart (Args() As String)
    Log(GetType(n))
End Sub
