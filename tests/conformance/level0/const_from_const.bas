Sub Process_Globals
    ' A Const whose value uses another constant (#33).
    Private Const Base As Int = 10
    Private Const Doubled As Int = Base * 2
End Sub

Sub AppStart (Args() As String)
    Log(Doubled)
End Sub
