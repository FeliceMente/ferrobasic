Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Dim of another type than Int after a For over the same undeclared
    ' name, which the For declares an Int: refused, as a For Each of
    ' another type is (local_typed_for_each_other_type_after_counter_named_like_sub)?
    ' (docs/open-questions.md)
    For i = 1 To 2
    Next
    Dim i As Double = 2.5
    Log(i)
End Sub
