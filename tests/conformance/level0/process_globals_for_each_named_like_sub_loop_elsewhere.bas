Sub Process_Globals
    ' A For Each variable of Process_Globals named like a Sub, looped over
    ' by a For in another Sub: accepted, as for a For counter, or refused?
    ' FerroBASIC rejects it until a recording shows.
    For Each Bump As Int In Array As Int(1)
    Next
End Sub

Sub AppStart (Args() As String)
    For Bump = 5 To 6
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
