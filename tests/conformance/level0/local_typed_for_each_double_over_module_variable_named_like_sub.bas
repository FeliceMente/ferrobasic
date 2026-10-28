Sub Process_Globals
    For Each Bump As Double In Array As Double(1.5)
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For Each As Double in a Sub over a name like a Sub's that a For
    ' Each As Double of Process_Globals declares: accepted, as with As Int?
    ' FerroBASIC rejects it until a recording shows.
    For Each Bump As Double In Array As Double(2.5)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
