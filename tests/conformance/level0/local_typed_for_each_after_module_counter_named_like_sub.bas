Sub Process_Globals
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For over a counter of Process_Globals named like a Sub, then a For
    ' Each with As over the same name: does the For count the module
    ' variable, and is the For Each accepted? FerroBASIC rejects it until a
    ' recording shows.
    For Bump = 5 To 6
    Next
    For Each Bump As Int In Array As Int(7)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
