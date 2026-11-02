Sub Process_Globals
    ' A For Each without `As` in Process_Globals over a variable declared
    ' there: refused as in a Sub (for_each_untyped_local)?
    ' (docs/open-questions.md)
    Private n As Int
    For Each n In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
