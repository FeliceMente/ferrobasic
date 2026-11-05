Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is `Until` accepted as a variable's name, as `In` is
    ' (docs/open-questions.md)?
    Dim Until As Int = 4
    Log(Until + 1)
End Sub
