Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim x As Int = 55
    Dim y As Int = 10
    Log(x / y)
    Dim z As Int = x / y
    Log(z)
    Log(Floor(x / y))
    Dim n As Int = -1
    Log(n Mod 24)
    Log(17 Mod 5)
End Sub
