Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does `1 < 2 < 3` compile, and how does it group
    ' (docs/open-questions.md)?
    Log(3 < 2 < 1)
End Sub
