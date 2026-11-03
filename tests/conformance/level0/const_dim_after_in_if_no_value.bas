Sub Process_Globals
    ' A Const, then a Dim of its name with no value inside an If: built,
    ' and reset to 0, as outside an If (const_dim_after_no_value)?
    ' (docs/open-questions.md)
    Private Const Size As Int = 10
    If True Then
        Private Size As Int
    End If
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
