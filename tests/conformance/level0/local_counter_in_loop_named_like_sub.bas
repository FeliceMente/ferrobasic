Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For inside a For Each As Int over the same name like a Sub's: does
    ' it share the variable, or is it refused? FerroBASIC rejects it until a
    ' recording shows.
    For Each Bump As Int In Array As Int(1)
        For Bump = 1 To 2
            Log(Bump)
        Next
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
