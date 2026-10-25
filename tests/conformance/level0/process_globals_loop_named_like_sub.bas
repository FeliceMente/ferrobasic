Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub: does the module
    ' variable hide the Sub elsewhere, or is it refused? FerroBASIC rejects
    ' it until a recording shows.
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Bump
End Sub

Sub Bump
    Log("bumped")
End Sub
