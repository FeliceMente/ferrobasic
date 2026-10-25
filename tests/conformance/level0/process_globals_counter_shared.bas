Sub Process_Globals
    Dim Count As Int
    ' If the undeclared counter of Process_Globals is a module variable, the
    ' loops of AppStart and Bump share it and this prints 2; if each is a
    ' local, 6. FerroBASIC rejects it until a recording shows.
    For i = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    For i = 1 To 3
        Bump
    Next
    Log(Count)
End Sub

Sub Bump
    For i = 1 To 2
        Count = Count + 1
    Next
End Sub
