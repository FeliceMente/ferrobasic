Sub Process_Globals
    ' A For Each without `As` in Process_Globals over a module variable
    ' Object that a typed loop there declared: accepted, as over one a Dim
    ' declared (process_globals_for_each_untyped_object)? FerroBASIC refuses
    ' it (docs/open-questions.md).
    For Each n As Object In Array As Int(1)
    Next
    For Each n In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
