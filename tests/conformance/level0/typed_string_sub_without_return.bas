Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What a String Sub returns when it ends without Return: an empty
    ' String, or Null (docs/open-questions.md)?
    Log("[" & NoString & "]")
End Sub

Sub NoString As String
End Sub
