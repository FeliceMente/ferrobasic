Sub Process_Globals
    For Each n As Object In Array As Int(1)
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` from a Sub over a module variable Object that
    ' a loop of Process_Globals declared: accepted, as over one a Dim
    ' declared (for_each_untyped_module_object)? FerroBASIC refuses it
    ' (docs/open-questions.md).
    For Each n In Array As Int(5, 6)
    Next
    Log(n)
End Sub
