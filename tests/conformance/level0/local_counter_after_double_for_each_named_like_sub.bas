Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For counter over a name like a Sub's after a For Each As Double
    ' declared it: does it count as a Double (3 steps of 0.5) or an Int?
    ' FerroBASIC rejects it until a recording shows.
    For Each Bump As Double In Array As Double(1.5)
        Log(Bump)
    Next
    Dim n As Int
    For Bump = 0 To 1 Step 0.5
        n = n + 1
    Next
    Log(n)
End Sub

Sub Bump As Int
    Return 9
End Sub
