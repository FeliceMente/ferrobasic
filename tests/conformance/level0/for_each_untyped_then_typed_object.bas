Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As`, which declares its variable an Object, then
    ' a For Each As Object over the same name: one variable, as with a Dim
    ' (for_each_untyped_object_declared_after)? FerroBASIC refuses it
    ' (docs/open-questions.md).
    For Each n In Array As Int(5, 6)
        Log(n)
    Next
    For Each n As Object In Array As Int(7)
    Next
    Log(n)
End Sub
