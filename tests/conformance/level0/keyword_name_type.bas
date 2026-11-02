Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is `Type` accepted as a variable's name, as `In` is
    ' (docs/open-questions.md)?
    Dim Type As Int = 4
    Log(Type + 1)
End Sub
