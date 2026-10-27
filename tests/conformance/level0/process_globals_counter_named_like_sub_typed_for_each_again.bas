Sub Process_Globals
    ' A For Each with As in Process_Globals over its own counter named like
    ' a Sub: accepted, as in a Sub over a For Each variable? FerroBASIC
    ' rejects it until a recording shows.
    For Bump = 1 To 2
    Next
    For Each Bump As Int In Array As Int(5)
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Bump)
End Sub

Sub Bump As Int
    Return 9
End Sub
