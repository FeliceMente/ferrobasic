Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For counter named like a Sub, then a For Each As Double over the
    ' same name: does the counter count as an Int, or as the Double the For
    ' Each declares (3 steps of 0.5 instead of 1)? FerroBASIC rejects it
    ' until a recording shows.
    Dim n As Int
    For Bump = 0 To 1 Step 0.5
        n = n + 1
    Next
    Log(n)
    For Each Bump As Double In Array As Double(1.5)
        Log(Bump)
    Next
End Sub

Sub Bump As Int
    Return 9
End Sub
