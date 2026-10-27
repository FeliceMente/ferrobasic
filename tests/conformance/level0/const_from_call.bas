Sub Process_Globals
    ' A Const whose value comes from a function call (#33). Rnd(1, 2) is
    ' always 1.
    Private Const Pick As Int = Rnd(1, 2)
End Sub

Sub AppStart (Args() As String)
    Log(Pick)
End Sub
