Sub Process_Globals
    Private n As Int
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` in a Sub over a module variable: refused as over
    ' a local (for_each_untyped_local)? (docs/open-questions.md)
    For Each n In Array As Int(5, 6)
    Next
    Log(n)
End Sub
