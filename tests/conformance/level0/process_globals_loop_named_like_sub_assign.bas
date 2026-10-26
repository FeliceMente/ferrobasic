Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub, assigned in
    ' another Sub: B4J reads the name as the Sub, and refuses the `=`.
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Bump = 5
    Log("assigned")
End Sub

Sub Bump As Int
    Return 9
End Sub
