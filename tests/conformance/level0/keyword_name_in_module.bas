Sub Process_Globals
    Private In As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept In as a module variable (docs/open-questions.md)? It
    ' is recorded only as a parameter (contextual_in). If it builds, this
    ' prints 6; otherwise B4J refuses it. FerroBASIC accepts it meanwhile.
    Log(In + 1)
End Sub
