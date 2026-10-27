Sub Process_Globals
    ' A For Each without As in Process_Globals over the variable of a typed
    ' For Each there named like a Sub: refused, as over its counter?
    ' FerroBASIC rejects it until a recording shows.
    For Each Bump As Int In Array As Int(1)
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
