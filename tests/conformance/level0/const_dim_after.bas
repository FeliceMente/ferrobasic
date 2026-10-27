Sub Process_Globals
    ' A Dim of a Const's name after it, with the same type: refused, the
    ' same constant, or a variable? FerroBASIC rejects it until a
    ' recording shows.
    Private Const Size As Int = 10
    Private Size As Int = 3
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
