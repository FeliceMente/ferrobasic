Sub Process_Globals
    ' A For inside a For of Process_Globals over the same name like a Sub's:
    ' accepted, sharing the counter, or refused? FerroBASIC rejects it until
    ' a recording shows.
    For Bump = 1 To 2
        For Bump = 1 To 2
            Log(Bump)
        Next
    Next
End Sub

Sub AppStart (Args() As String)
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
