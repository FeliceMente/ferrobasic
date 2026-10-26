Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Two For Each loops with As over a name that is a Sub's: accepted, as
    ' a For is, or refused, as a For Each without As is? FerroBASIC rejects
    ' it until a recording shows.
    For Each Bump As Int In Array As Int(1)
    Next
    For Each Bump As Int In Array As Int(2)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
