Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Show("a")
End Sub

Sub Show (S As String)
    ' A For Each without `As` over a parameter: refused as over a local
    ' (for_each_untyped_local)? (docs/open-questions.md)
    For Each S In Array As String("x", "y")
        Log(S)
    Next
End Sub
