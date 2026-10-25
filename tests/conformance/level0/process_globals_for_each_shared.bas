Sub Process_Globals
    Dim Count As Int
    ' If the For Each variable of Process_Globals is a module variable, the
    ' undeclared counters of AppStart and Bump share it and this prints 2; if
    ' it is a local, 6. FerroBASIC rejects it until a recording shows.
    For Each n As Int In Array As Int(1, 2)
    Next
End Sub

Sub AppStart (Args() As String)
    For n = 1 To 3
        Bump
    Next
    Log(Count)
End Sub

Sub Bump
    For n = 1 To 2
        Count = Count + 1
    Next
End Sub
