Sub Process_Globals
    ' A For Each As nested in a For Each As of Process_Globals over the same
    ' name like a Sub's: accepted, as in a Sub? FerroBASIC rejects it until a
    ' recording shows.
    For Each Bump As Int In Array As Int(1, 2)
        For Each Bump As Int In Array As Int(5)
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
