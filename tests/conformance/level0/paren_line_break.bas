Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does a line break inside parentheses continue the line without ` _`
    ' (docs/open-questions.md)?
    Log(Max(1,
        2))
End Sub
