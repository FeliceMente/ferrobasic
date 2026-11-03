Sub Process_Globals
    Private n As Object
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` in a Sub over a module variable of type
    ' Object: accepted, as over a local Object (for_each_untyped_object)?
    ' FerroBASIC refuses it (docs/open-questions.md).
    For Each n In Array As Int(5, 6)
    Next
    Log(n)
End Sub
