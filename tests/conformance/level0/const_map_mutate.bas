Sub Process_Globals
    ' Whether a Map constant's contents may change (#33).
    Private Const Prices As Map = CreateMap("x": 1)
End Sub

Sub AppStart (Args() As String)
    Prices.Put("y", 2)
    Log(Prices.Size)
End Sub
