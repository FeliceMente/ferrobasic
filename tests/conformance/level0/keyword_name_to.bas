Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is `To` accepted as a variable's name, as `In` is
    ' (docs/open-questions.md)?
    Dim To As Int = 4
    Log(To + 1)
End Sub
