Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Values sets an existing unseen key?
    ' Recording run 24 records that iteration completes and sees the new value:
    ' replacing a value is not a structural change. Adding or removing a
    ' key is (map_values_add_in_for_each, map_values_remove_in_for_each).
    ' FerroBASIC prints the same.
    ' See docs/spec/level0-builtins.md, Map.
    ' Expected output, one line per item below:
    '   seen=1
    '   seen=20
    '   seen=3
    '   complete
    '   size=3
    '   b=20
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Dim changed As Boolean = False
    Try
        For Each item As Int In m.Values
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
