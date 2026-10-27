Sub Process_Globals
    ' A For Each without As in Process_Globals over its own counter named
    ' like a Sub: refused, as in another Sub? FerroBASIC rejects it until a
    ' recording shows.
    For Bump = 1 To 2
    Next
    For Each Bump In Array As Int(5)
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
