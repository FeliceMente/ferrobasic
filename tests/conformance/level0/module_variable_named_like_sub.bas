Sub Process_Globals
    Dim Bump As Int = 4
End Sub

Sub AppStart (Args() As String)
    ' A module variable declared with a Sub's name, read as a value: the
    ' variable (this prints 4) or the Sub (9)? FerroBASIC rejects it until a
    ' recording shows.
    Log(Bump)
End Sub

Sub Bump As Int
    Return 9
End Sub
