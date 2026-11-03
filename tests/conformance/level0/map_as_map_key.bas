Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is a Map used as a Map key compared by identity or by its contents
    ' (docs/open-questions.md)? A List is compared by its contents
    ' (level0/list_as_map_key).
    Dim a As Map
    a.Initialize
    a.Put("k", 1)
    Dim b As Map
    b.Initialize
    b.Put("k", 1)
    Dim m As Map
    m.Initialize
    m.Put(a, "a")
    Log(m.ContainsKey(b))
    m.Put(b, "b")
    Log(m.Size)
    ' A Map inside a List key.
    Dim x As List
    x.Initialize
    x.Add(a)
    Dim y As List
    y.Initialize
    y.Add(b)
    m.Put(x, "x")
    Log(m.ContainsKey(y))
End Sub
