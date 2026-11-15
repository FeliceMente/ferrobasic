Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept Each as the counter of a For (docs/open-questions.md)?
    ' It is recorded only as a local variable (keyword_name_each). If it
    ' builds, this prints 1 and 2; otherwise B4J refuses it. FerroBASIC
    ' refuses it meanwhile: its parser reads `For Each` as the start of a
    ' For Each loop.
    For Each = 1 To 2
        Log(Each)
    Next
End Sub
