Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What a typed Sub returns when it ends without Return
    ' (docs/open-questions.md).
    Log(NoInt)
    Log(NoString)
    Log(NoString = Null)
End Sub

Sub NoInt As Int
End Sub

Sub NoString As String
End Sub
