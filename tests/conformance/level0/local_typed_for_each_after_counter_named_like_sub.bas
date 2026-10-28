Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each with As over a For counter of the same Sub named like a
    ' Sub: accepted, as over a For Each variable? FerroBASIC rejects it
    ' until a recording shows.
    For Bump = 1 To 2
    Next
    For Each Bump As Int In Array As Int(5)
        Log(Bump)
    Next
    Log(Bump)
End Sub

Sub Bump As Int
    Return 9
End Sub
