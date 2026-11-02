Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is `Each` accepted as a variable's name, as `In` is
    ' (docs/open-questions.md)?
    Dim Each As Int = 4
    Log(Each + 1)
End Sub
