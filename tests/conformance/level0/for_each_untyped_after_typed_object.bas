Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` over an Object that a typed loop declared,
    ' not a Dim: accepted as over a local Object (for_each_untyped_object)?
    ' FerroBASIC refuses it (docs/open-questions.md).
    For Each u As Object In Array As Int(1)
    Next
    For Each u In Array As Int(7)
    Next
    Log(u)
End Sub
