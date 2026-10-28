Sub Process_Globals
    ' A Const of a variable's name after its Dim, with the same type:
    ' refused, or the variable made constant? FerroBASIC rejects it until a
    ' recording shows.
    Private Size As Int = 3
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
