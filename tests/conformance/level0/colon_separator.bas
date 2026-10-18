Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim a As Int = 0
    Dim c As Int = 0
    Dim b As Int = 1
    If b = 0 Then a = 1 : c = 1
    Log(a)
    Log(c)
    Log("x") : Log("y")
End Sub
