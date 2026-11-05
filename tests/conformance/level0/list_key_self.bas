Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A List that holds itself, used as a Map key: Java's hashCode recurses
    ' (docs/open-questions.md).
    Dim a As List
    a.Initialize
    a.Add(a)
    Dim m As Map
    m.Initialize
    Log("before")
    m.Put(a, 1)
    Log(m.Size)
End Sub
