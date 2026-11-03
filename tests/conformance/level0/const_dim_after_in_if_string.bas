Sub Process_Globals
    ' A String Const, then a Dim of its name inside an If: built, as an Int
    ' (const_dim_after_in_if)? (docs/open-questions.md)
    Private Const Name As String = "a"
    If True Then
        Private Name As String = "b"
    End If
End Sub

Sub AppStart (Args() As String)
    Log(Name)
End Sub
