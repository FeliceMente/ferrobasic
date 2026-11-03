Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As`, which declares its variable an Object
    ' (for_each_untyped_declared_after), then a Dim of the same name as an
    ' Object: one variable? FerroBASIC refuses it (docs/open-questions.md).
    For Each n In Array As Int(5, 6)
        Log(n)
    Next
    Dim n As Object
    Log(n)
End Sub
