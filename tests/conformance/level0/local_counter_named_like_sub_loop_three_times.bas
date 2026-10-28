Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Three For loops over a name like a Sub's, one after the other:
    ' accepted, as two are? FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
    Next
    For Bump = 1 To 2
    Next
    For Bump = 1 To 2
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
