Sub Process_Globals
    ' A Double constant (#33).
    Private Const Rate As Double = 0.25
End Sub

Sub AppStart (Args() As String)
    Log(Rate * 10)
End Sub
