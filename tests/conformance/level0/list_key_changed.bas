Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A List changed after it was used as a Map key (docs/open-questions.md):
    ' Java's HashMap keeps the hash it had when it was put, and compares
    ' its contents now.
    Dim a As List
    a.Initialize
    a.Add(1)
    Dim m As Map
    m.Initialize
    m.Put(a, "a")
    a.Add(2)
    Dim old As List
    old.Initialize
    old.Add(1)
    Log(m.ContainsKey(a))
    Log(m.ContainsKey(old))
    m.Put(old, "old")
    Log(m.Size)
    a.RemoveAt(1)
    Log(m.Get(old))
    m.Remove(a)
    Log(m.Get(old))
    Log(m)
End Sub
