Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Keys adds a key at its last item?
    ' Recording run 24 shows ConcurrentModificationException at the item after
    ' an addition (map_keys_add_in_for_each). Here no item follows: complete if
    ' the loop ends over the items it had, seen=d too if it visits the new key,
    ' or a caught error's kind tests (cme/nse/npe/bounds). size=4 afterwards.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   seen=a
    '   seen=b
    '   seen=c
    '   complete
    '   size=4
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Try
        For Each item As String In m.Keys
            Log("seen=" & item)
            If item = "c" Then m.Put("d", 4)
        Next
        Log("complete")
    Catch
        Log("cme=" & LastException.Message.Contains("ConcurrentModificationException"))
        Log("nse=" & LastException.Message.Contains("NoSuchElementException"))
        Log("npe=" & LastException.Message.Contains("NullPointerException"))
        Log("bounds=" & LastException.Message.Contains("IndexOutOfBoundsException"))
    End Try
    Log("size=" & m.Size)
End Sub
