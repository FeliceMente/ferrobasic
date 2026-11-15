Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept Each as the name of a Sub (docs/open-questions.md)? It
    ' is recorded only as a local variable (keyword_name_each). If it
    ' builds, this prints 8; otherwise B4J refuses it. FerroBASIC accepts it
    ' meanwhile.
    Log(Each + 1)
End Sub

Sub Each As Int
    Return 7
End Sub
