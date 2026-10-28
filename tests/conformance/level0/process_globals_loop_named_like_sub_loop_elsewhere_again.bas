Sub Process_Globals
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    ' Two For loops in another Sub over a counter of Process_Globals named
    ' like a Sub: accepted, as one is? FerroBASIC rejects it until a
    ' recording shows.
    For Bump = 5 To 6
    Next
    For Bump = 7 To 8
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
