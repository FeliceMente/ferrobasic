Sub Process_Globals
    ' A Const, then a Dim of its name with no value: may a Sub then assign
    ' it, as after a Dim with a value (const_dim_after_assign)?
    ' (docs/open-questions.md)
    Private Const Size As Int = 10
    Private Size As Int
End Sub

Sub AppStart (Args() As String)
    Size = 5
    Log(Size)
End Sub
