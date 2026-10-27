Sub Process_Globals
    ' A Const whose value comes from a call, not a constant expression
    ' (#33). Rnd(1, 2) is always 1.
    Private Const Pick As Int = Rnd(1, 2)
    Private Const Length As Int = "abc".Length
End Sub

Sub AppStart (Args() As String)
    Log(Pick)
    Log(Length)
End Sub
