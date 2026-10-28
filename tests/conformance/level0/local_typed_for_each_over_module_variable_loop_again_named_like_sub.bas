Sub Process_Globals
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    ' Two For Each As Int in a Sub over a name like a Sub's that a counter
    ' of Process_Globals declares too: accepted, as one is? FerroBASIC
    ' rejects it until a recording shows.
    For Each Bump As Int In Array As Int(5)
    Next
    For Each Bump As Int In Array As Int(6)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
