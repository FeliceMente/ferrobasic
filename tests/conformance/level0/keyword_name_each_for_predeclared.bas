Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Run 22 refused keyword_name_each_for at Log(Each), where Each
    ' had no declaration. Does declaring Each first let this build?
    ' The loop body would print 1 and 2 if Each is its counter.
    ' The final Log also shows what happened to the declared variable.
    ' FerroBASIC currently rejects the header (docs/open-questions.md).
    Dim Each As Int = 9
    For Each = 1 To 2
        Log(Each)
    Next
    Log(Each)
End Sub
