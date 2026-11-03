Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each As String over a name like a Sub's, twice: accepted, as with
    ' Int and Double (local_typed_for_each_named_like_sub_loop_again,
    ' local_typed_for_each_double_loop_again_named_like_sub)?
    ' (docs/open-questions.md)
    For Each Bump As String In Array As String("a")
    Next
    For Each Bump As String In Array As String("b")
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
