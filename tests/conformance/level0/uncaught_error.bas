Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Log("before")
    Dim m As Map
    m.Put("k", 1)
    Log("not reached")
End Sub
