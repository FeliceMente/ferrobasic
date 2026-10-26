Sub Process_Globals
    Dim Count As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' A For Each in a Sub declaring a module variable's name and type: the
    ' module variable (this prints 7) or a new local (5)? FerroBASIC rejects
    ' it until a recording shows.
    For Each Count As Int In Array As Int(6, 7)
    Next
    Show
End Sub

Sub Show
    Log(Count)
End Sub
