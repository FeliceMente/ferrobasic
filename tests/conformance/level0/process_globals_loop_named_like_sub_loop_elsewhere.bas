Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, looped over in
    ' another Sub: accepted, or refused as an assignment is? FerroBASIC
    ' rejects it until a recording shows.
    For Bump = 1 To 2
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
