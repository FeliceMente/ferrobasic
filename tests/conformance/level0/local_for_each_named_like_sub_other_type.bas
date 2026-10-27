Sub Process_Globals
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For Each of another type than a module variable named like a Sub:
    ' refused as for any other name ("Current declaration does not match
    ' previous one"), or accepted since the name is the Sub? FerroBASIC
    ' rejects it until a recording shows.
    For Each Bump As String In Array As String("x")
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
