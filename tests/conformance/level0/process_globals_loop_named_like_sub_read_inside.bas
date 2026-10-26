Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, read inside its
    ' own loop: the variable (this prints 1, then 2) or the Sub (9, 9)?
    ' FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
End Sub

Sub Bump As Int
    Return 9
End Sub
