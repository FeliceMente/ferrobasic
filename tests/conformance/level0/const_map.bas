Sub Process_Globals
    ' A Const of type Map (#33).
    Private Const Prices As Map = CreateMap("x": 1)
End Sub

Sub AppStart (Args() As String)
    Log(Prices.Get("x"))
End Sub
