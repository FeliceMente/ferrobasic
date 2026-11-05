Sub Process_Globals
    ' A For Each without `As` in Process_Globals over a Type's name nothing
    ' else declares: a module variable Object, as over another undeclared
    ' name (process_globals_for_each_untyped_undeclared) and as a counter
    ' there may take a Type's name (process_globals_loop_named_like_type)?
    ' FerroBASIC refuses it (docs/open-questions.md).
    Type Point (X As Int)
    For Each Point In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Point)
End Sub
