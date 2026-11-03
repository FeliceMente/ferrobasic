Sub Process_Globals
    ' A Const, then a Dim of its name inside an If that does not run: does
    ' the constant keep its value, 10, or does the declaration reset it
    ' (docs/open-questions.md)?
    Private Const Size As Int = 10
    If False Then
        Private Size As Int = 3
    End If
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
