Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without As inside a For over the same variable named like
    ' a Sub: refused, as one after the other? FerroBASIC rejects it until a
    ' recording shows.
    For Bump = 1 To 2
        For Each Bump In Array As Int(5)
            Log(Bump)
        Next
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
