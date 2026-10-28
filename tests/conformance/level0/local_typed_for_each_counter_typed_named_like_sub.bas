Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each As Int, a For, then a For Each As Int over a name like a
    ' Sub's: accepted, as each pair is? FerroBASIC rejects it until a
    ' recording shows.
    For Each Bump As Int In Array As Int(1)
    Next
    For Bump = 1 To 2
    Next
    For Each Bump As Int In Array As Int(5)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
