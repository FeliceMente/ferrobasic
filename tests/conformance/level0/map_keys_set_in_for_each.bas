Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Keys sets an existing unseen key?
    ' Recording run 24 records that iteration completes: replacing the value of a key
    ' is not a structural change, so no ConcurrentModificationException.
    ' Adding or removing a key is (map_keys_add_in_for_each,
    ' map_keys_remove_in_for_each). FerroBASIC prints the same.
    ' See docs/spec/level0-builtins.md, Map.
    ' Expected output, one line per item below:
    '   seen=a
    '   seen=b
    '   seen=c
    '   complete
    '   size=3
    '   b=20
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Dim changed As Boolean = False
    Try
        For Each item As String In m.Keys
            Log("seen=" & item)
            If Not(changed) Then
                changed = True
                m.Put("b", 20)
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
