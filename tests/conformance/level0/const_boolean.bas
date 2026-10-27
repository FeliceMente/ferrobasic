Sub Process_Globals
    ' A Boolean constant (#33).
    Private Const Enabled As Boolean = True
End Sub

Sub AppStart (Args() As String)
    Log(Enabled)
End Sub
