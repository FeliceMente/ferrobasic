Sub Process_Globals
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    ' A For Each of the same type as a module variable is that variable;
    ' when the name is also a Sub's, is it, or a new local, or refused?
    ' FerroBASIC rejects it until a recording shows.
    For Each Bump As Int In Array As Int(7)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
