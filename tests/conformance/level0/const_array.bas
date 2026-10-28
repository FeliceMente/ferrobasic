Sub Process_Globals
    ' A Const array (#33).
    Private Const Days() As String = Array As String("Mon", "Tue")
End Sub

Sub AppStart (Args() As String)
    Log(Days(1))
End Sub
