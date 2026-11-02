Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What an Int Sub returns when it ends without Return
    ' (docs/open-questions.md).
    Log(NoInt)
End Sub

Sub NoInt As Int
End Sub
