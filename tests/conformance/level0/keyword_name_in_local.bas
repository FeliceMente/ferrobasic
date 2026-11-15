Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept In as a local variable (docs/open-questions.md)? It is
    ' recorded only as a parameter (contextual_in). If it builds, this
    ' prints 4; otherwise B4J refuses it. FerroBASIC accepts it meanwhile.
    Dim In As Int = 3
    Log(In + 1)
End Sub
