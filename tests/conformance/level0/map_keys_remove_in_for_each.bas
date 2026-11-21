Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Keys removes an unseen key?
    ' Recording run 24: B4J raises ConcurrentModificationException at the next
    ' item. Adding or removing a key changes the map's modCount, which Java's
    ' iterator checks; replacing a value does not (map_keys_set_in_for_each).
    ' FerroBASIC prints the same.
    ' See docs/spec/level0-builtins.md, Map.
    ' Expected output, one line per item below:
    '   seen=a
    '   cme=true
    '   nse=false
    '   npe=false
    '   bounds=false
    '   size=2
    '   b=null
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Dim changed As Boolean = False
    Try
        For Each item As String In m.Keys
            Log("seen=" & item)
            If Not(changed) Then
                changed = True
                m.Remove("b")
            End If
        Next
        Log("complete")
    Catch
        Log("cme=" & LastException.Message.Contains("ConcurrentModificationException"))
        Log("nse=" & LastException.Message.Contains("NoSuchElementException"))
        Log("npe=" & LastException.Message.Contains("NullPointerException"))
        Log("bounds=" & LastException.Message.Contains("IndexOutOfBoundsException"))
    End Try
    Log("size=" & m.Size)
    Log("b=" & m.Get("b"))
End Sub
