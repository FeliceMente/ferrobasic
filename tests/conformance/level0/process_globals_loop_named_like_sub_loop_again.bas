Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, looped over
    ' again in Process_Globals: accepted, and is Bump the Sub inside the
    ' second loop? FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
    Next
    For Bump = 1 To 2
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
End Sub

Sub Bump As Int
    Return 9
End Sub
