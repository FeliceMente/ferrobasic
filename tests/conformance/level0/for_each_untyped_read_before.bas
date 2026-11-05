Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A name read, not assigned, before a For Each without `As` over it:
    ' refused as after an assignment (for_each_untyped_assigned_before), or
    ' does the loop declare it? FerroBASIC refuses it (docs/open-questions.md).
    Log(u)
    For Each u In Array As Int(7)
    Next
    Log(u)
End Sub
