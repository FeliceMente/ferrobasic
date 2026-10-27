Sub Process_Globals
    For Each Bump As String In Array As String("x")
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For in another Sub over a String loop variable of Process_Globals
    ' named like a Sub: accepted, the name being the Sub, or refused as a
    ' String counter? FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
