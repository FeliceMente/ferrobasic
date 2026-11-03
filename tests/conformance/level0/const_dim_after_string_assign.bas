Sub Process_Globals
    ' A String Const, then a Dim of its name: may a Sub then assign it, as
    ' an Int (const_dim_after_assign)? (docs/open-questions.md)
    Private Const Name As String = "a"
    Private Name As String = "b"
End Sub

Sub AppStart (Args() As String)
    Name = "c"
    Log(Name)
End Sub
