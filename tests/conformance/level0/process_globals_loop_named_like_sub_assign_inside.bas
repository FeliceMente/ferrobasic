Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, assigned inside
    ' its own loop: the variable (this prints 1), or refused as in another
    ' Sub? FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
        Bump = 3
    Next
End Sub

Sub AppStart (Args() As String)
    Log("assigned")
End Sub

Sub Bump As Int
    Return 9
End Sub
