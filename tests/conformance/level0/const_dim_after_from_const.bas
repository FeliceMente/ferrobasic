Sub Process_Globals
    ' A Dim of a Const's name after it, whose value reads the Const: built
    ' as with a literal (const_dim_after), and is it 11? FerroBASIC rejects
    ' it until a recording shows.
    Private Const Size As Int = 10
    Private Size As Int = Size + 1
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
