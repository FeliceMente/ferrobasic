Sub Process_Globals
    ' Whether an array constant's items may change (#33).
    Private Const Days() As String = Array As String("Mon", "Tue")
End Sub

Sub AppStart (Args() As String)
    Days(0) = "Sun"
    Log(Days(0))
End Sub
