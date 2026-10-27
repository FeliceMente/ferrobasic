Sub Process_Globals
    ' A Const with no value. The B4A 4.00 notes say a constant is assigned
    ' when declared; whether one without a value is refused is to be
    ' recorded (#33).
    Private Const Size As Int
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
