Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is a parameter without `As` a String (docs/open-questions.md)?
    Show(5)
    Show(2.50)
End Sub

Sub Show (X)
    Log(GetType(X))
    Log(X)
End Sub
