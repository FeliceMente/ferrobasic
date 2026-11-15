Sub Process_Globals
    Private Each As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept Each as a module variable (docs/open-questions.md)? It
    ' is recorded only as a local variable (keyword_name_each). If it
    ' builds, this prints 6; otherwise B4J refuses it. FerroBASIC accepts it
    ' meanwhile.
    Log(Each + 1)
End Sub
