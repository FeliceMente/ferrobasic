Sub Process_Globals
    ' A For Each without As nested in a For of Process_Globals over the
    ' same name like a Sub's: refused, as in a Sub? FerroBASIC rejects it
    ' until a recording shows.
    For Bump = 1 To 2
        For Each Bump In Array As Int(5)
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
