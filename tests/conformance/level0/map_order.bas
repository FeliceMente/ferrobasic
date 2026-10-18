Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim m As Map
    m.Initialize
    m.Put("b", 1)
    m.Put("a", 2)
    m.Put("c", 3)
    m.Put("b", 4)
    For Each k As String In m.Keys
        Log(k & "=" & m.Get(k))
    Next
    Log(m.Size)
    Log(m.ContainsKey("B"))
    Log(m.GetDefault("z", "none"))
End Sub
