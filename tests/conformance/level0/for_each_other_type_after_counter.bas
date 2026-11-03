Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each As Double after a For over the same undeclared name, which
    ' the For declares an Int: refused, as with a name of a Sub
    ' (local_typed_for_each_other_type_after_counter_named_like_sub)?
    ' (docs/open-questions.md)
    For i = 1 To 2
    Next
    For Each i As Double In Array As Double(1.5)
        Log(i)
    Next
End Sub
