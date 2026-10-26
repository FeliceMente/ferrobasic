Sub Process_Globals
    ' A loop variable of Process_Globals named like a Sub that takes an
    ' argument, used with one in another Sub: the Sub (this prints 10), or
    ' refused? FerroBASIC rejects it until a recording shows.
    For Bump = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Bump(1))
End Sub

Sub Bump (X As Int) As Int
    Return X * 10
End Sub
