Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept In as the name of a Sub (docs/open-questions.md)? It
    ' is recorded only as a parameter (contextual_in). If it builds, this
    ' prints 8; otherwise B4J refuses it. FerroBASIC accepts it meanwhile.
    Log(In + 1)
End Sub

Sub In As Int
    Return 7
End Sub
