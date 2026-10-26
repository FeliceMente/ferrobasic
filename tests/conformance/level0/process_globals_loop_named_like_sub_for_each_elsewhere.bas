Sub Process_Globals
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For Each without As in another Sub, over a loop variable of
    ' Process_Globals named like a Sub: accepted, as a For is? FerroBASIC
    ' rejects it until a recording shows.
    For Each Bump In Array As Int(5)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
