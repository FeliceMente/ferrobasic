Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For counter named like a Sub, looped over again in the same Sub:
    ' accepted? FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
    Next
    For Bump = 1 To 2
        Log(Bump)
    Next
End Sub

Sub Bump As Int
    Return 9
End Sub
