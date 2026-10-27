Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Log(m.GetKeyAt(0) & m.GetKeyAt(1) & m.GetKeyAt(2))
    Try
        Log(m.GetKeyAt(3))
    Catch
        Log("past the end")
    End Try
    Log(m.GetKeyAt(2))
    Log(m.GetKeyAt(0))
    Try
        Log(m.GetKeyAt(5))
    Catch
        Log("past the end")
    End Try
    Log(m.GetKeyAt(0))
    Try
        Log(m.GetKeyAt(1))
    Catch
        Log("used up")
    End Try
    Dim n As Map = CreateMap("x": 1)
    Try
        Log(n.GetKeyAt(-1))
    Catch
        Log("no entry yet")
    End Try
    Log(n.GetKeyAt(0))
    Log(n.GetKeyAt(-1))
    Dim r As Map = CreateMap("p": 1, "q": 2)
    Log(r.GetValueAt(1))
    r.Remove("q")
    Log(r.GetKeyAt(-1) & r.GetValueAt(-1))
    r.Put("q", 5)
    Log(r.GetKeyAt(-1) & r.GetValueAt(-1))
    Dim u As Map = CreateMap("k": 1)
    Log(u.GetValueAt(0))
    u.Put("k", 9)
    Log(u.GetValueAt(-1))
    u.Clear
    Log(u.GetKeyAt(-7) & u.GetValueAt(-7))
    ' Entries before the last one returned can go: it stays the same entry.
    Dim w As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Log(w.GetKeyAt(2))
    w.Remove("a")
    Log(w.GetKeyAt(-1) & w.GetKeyAt(1))
    ' Any change drops the iterator, even one that ran past the end.
    Log(w.GetKeyAt(0))
    Try
        Log(w.GetKeyAt(9))
    Catch
        Log("past the end")
    End Try
    w.Put("d", 4)
    Log(w.GetKeyAt(0))
End Sub
