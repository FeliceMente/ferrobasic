Sub Process_Globals
    ' A Const whose value is arithmetic on literals (#33).
    Private Const Total As Int = 2 * 5 + 1
End Sub

Sub AppStart (Args() As String)
    Log(Total)
End Sub
