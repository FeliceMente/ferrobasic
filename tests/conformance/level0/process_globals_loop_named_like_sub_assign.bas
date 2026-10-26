Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, assigned in
    ' another Sub: accepted, writing the variable, or refused? FerroBASIC
    ' rejects it until a recording shows.
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Bump = 5
    Log("assigned")
End Sub

Sub Bump As Int
    Return 9
End Sub
