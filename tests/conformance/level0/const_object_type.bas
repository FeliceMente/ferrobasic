Sub Process_Globals
    ' Constants of object types: a List, a Map and an array (#33).
    Private Const Names As List = Array("a", "b")
    Private Const Prices As Map = CreateMap("x": 1)
    Private Const Days() As String = Array As String("Mon", "Tue")
End Sub

Sub AppStart (Args() As String)
    Log(Names.Size)
    Log(Prices.Get("x"))
    Log(Days(1))
End Sub
