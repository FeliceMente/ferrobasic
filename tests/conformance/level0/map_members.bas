Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim m As Map = CreateMap("one": "uno", "two": "due", "three": "tre")
    Log(m.Size)
    Log(m.Get("two") & " " & m.Get("four"))
    Log(m.GetDefault("four", "quattro") & " " & m.GetDefault("one", "?"))
    ' Keys are compared with equals: case matters.
    Log(m.ContainsKey("one") & " " & m.ContainsKey("One"))
    ' Remove returns the removed value, or Null.
    Log(m.Remove("two") & " " & m.Remove("nine"))
    ' Replacing a value keeps the key's position; a new key goes last.
    m.Put("one", "eins")
    m.Put("four", "vier")
    For Each k As String In m.Keys
        Log(k & "=" & m.Get(k))
    Next
    Dim all As String = ""
    For Each v As String In m.Values
        all = all & v & " "
    Next
    Log(all.Trim)
    ' A key whose value is Null is present, and GetDefault returns the default.
    m.Put("none", Null)
    Log(m.ContainsKey("none") & " " & m.GetDefault("none", "default") & " " & m.Size)
    m.Clear
    Log(m.Size & " " & m.ContainsKey("one"))
    Dim fresh As Map
    fresh.Initialize
    Log(fresh.Size)
End Sub
