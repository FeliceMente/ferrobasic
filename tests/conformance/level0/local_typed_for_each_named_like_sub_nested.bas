Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each with As inside a For Each with As over the same variable
    ' named like a Sub: accepted, as one after the other? FerroBASIC rejects
    ' it until a recording shows.
    For Each Bump As Int In Array As Int(1, 2)
        For Each Bump As Int In Array As Int(5)
            Log(Bump)
        Next
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
