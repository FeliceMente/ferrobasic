Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Keys adds a key?
    ' One seen= line per item: a,b,c for a snapshot; a,b,c,d for a live view; complete if
    ' iteration ends. A caught error prints kind tests (cme/nse/npe/bounds); size=4 afterwards
    ' if mutation ran. b=2 after adding the new key.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   seen=a
    '   seen=b
    '   seen=c
    '   complete
    '   size=4
    '   b=2
    Dim m As Map = CreateMap("a": 1, "b": 2, "c": 3)
    Dim changed As Boolean = False
    Try
        For Each item As String In m.Keys
            Log("seen=" & item)
            If Not(changed) Then
                changed = True
                m.Put("d", 4)
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
