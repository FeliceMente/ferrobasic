Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is `Step` accepted as a variable's name, as `In` is
    ' (docs/open-questions.md)?
    Dim Step As Int = 4
    Log(Step + 1)
End Sub
