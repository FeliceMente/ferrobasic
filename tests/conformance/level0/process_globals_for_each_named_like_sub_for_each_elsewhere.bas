Sub Process_Globals
    For Each Bump As Int In Array As Int(1)
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For Each without As in another Sub over a For Each variable of
    ' Process_Globals named like a Sub: refused, as over a counter there?
    ' FerroBASIC rejects it until a recording shows.
    For Each Bump In Array As Int(5)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
