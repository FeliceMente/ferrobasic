Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Keys clears the map at its first item?
    ' Recording run 24 shows ConcurrentModificationException at the item after
    ' an addition or a removal (map_keys_add_in_for_each). cme=true if Clear
    ' counts as one; complete after seen=a if the loop just ends; otherwise the
    ' seen= lines and kind tests (cme/nse/npe/bounds) show what it does. size=0.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   seen=a
    '   cme=true
    '   nse=false
    '   npe=false
    '   bounds=false
    '   size=0
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Try
        For Each item As String In m.Keys
            Log("seen=" & item)
            If item = "a" Then m.Clear
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
