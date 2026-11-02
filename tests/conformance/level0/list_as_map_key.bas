Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is a List used as a Map key compared by identity or by its contents
    ' (docs/open-questions.md)?
    Dim a As List
    a.Initialize
    a.Add(1)
    Dim b As List
    b.Initialize
    b.Add(1)
    Dim m As Map
    m.Initialize
    m.Put(a, "a")
    Log(m.ContainsKey(b))
    m.Put(b, "b")
    Log(m.Size)
End Sub
