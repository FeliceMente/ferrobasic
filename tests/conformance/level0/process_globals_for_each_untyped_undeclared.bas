Sub Process_Globals
    ' A For Each without `As` in Process_Globals over a name nothing
    ' declares: a module variable Object, as a typed loop there declares one
    ' (process_globals_for_each), or as the untyped loop declares a local in
    ' a Sub (for_each_untyped_undeclared)? A module variable, which AppStart
    ' reads: recording run 20.
    For Each u In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(u)
End Sub
