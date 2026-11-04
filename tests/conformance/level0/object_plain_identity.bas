Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Two new Objects (object_dim_default) compare by identity, as keys of
    ' a Map and in a List too (docs/open-questions.md)?
    Dim o As Object
    Dim p As Object
    Log(o = o)
    Log(o = p)
    Log(o <> p)
    Dim m As Map
    m.Initialize
    m.Put(o, 1)
    Log(m.ContainsKey(o))
    Log(m.ContainsKey(p))
    Dim l As List
    l.Initialize
    l.Add(o)
    Log(l.IndexOf(o))
    Log(l.IndexOf(p))
    ' Its text is java.lang.Object@<hash>, which changes on every run.
    Dim s As String = o
    Log(s.StartsWith("java.lang.Object@"))
End Sub
