Sub Process_Globals
    ' A Long constant, from a literal beyond the Int range (#33).
    Private Const Big As Long = 9000000000
End Sub

Sub AppStart (Args() As String)
    Log(Big)
End Sub
