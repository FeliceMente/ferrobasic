Sub Process_Globals
    ' Three For loops of Process_Globals over a name like a Sub's, one
    ' after the other: accepted, as two are? FerroBASIC rejects it until a
    ' recording shows.
    For Bump = 1 To 2
    Next
    For Bump = 1 To 2
    Next
    For Bump = 1 To 2
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
