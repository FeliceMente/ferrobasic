Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` over a name never declared: refused, or does it
    ' declare it, as an undeclared For counter does (docs/open-questions.md)?
    For Each u In Array As Int(5, 6)
        Log(u)
    Next
End Sub
