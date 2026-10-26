Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each variable named like a Sub, looped over again by a For Each
    ' without As in the same Sub: accepted, as a For is? FerroBASIC rejects
    ' it until a recording shows.
    For Each Bump As Int In Array As Int(1)
    Next
    For Each Bump In Array As Int(2)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
