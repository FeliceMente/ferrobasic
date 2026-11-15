Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J build a counted loop with Each and an explicit type?
    ' The loop body would print 1 and 2 if Each is its counter.
    ' The final Log checks whether the header declares Each.
    ' FerroBASIC currently rejects the header (docs/open-questions.md).
    For Each As Int = 1 To 2
        Log(Each)
    Next
    Log(Each)
End Sub
