Sub Process_Globals
    ' A For over a For Each variable of Process_Globals named like a Sub,
    ' in Process_Globals too: accepted, as over a For counter? FerroBASIC
    ' rejects it until a recording shows.
    For Each Bump As Int In Array As Int(1)
    Next
    For Bump = 1 To 2
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
End Sub

Sub Bump As Int
    Return 9
End Sub
