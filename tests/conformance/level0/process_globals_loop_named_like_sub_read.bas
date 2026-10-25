Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, read as a value
    ' elsewhere: the variable (this prints 3) or the Sub (9)? FerroBASIC
    ' rejects it until a recording shows.
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Bump)
End Sub

Sub Bump As Int
    Return 9
End Sub
