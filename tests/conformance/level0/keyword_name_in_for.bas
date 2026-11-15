Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept In as the counter of a For (docs/open-questions.md)?
    ' It is recorded only as a parameter (contextual_in). If it builds, this
    ' prints 1 and 2; otherwise B4J refuses it. FerroBASIC accepts it
    ' meanwhile.
    For In = 1 To 2
        Log(In)
    Next
End Sub
