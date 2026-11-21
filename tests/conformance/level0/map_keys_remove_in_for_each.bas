Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' What happens when For Each over Map.Keys removes an unseen key?
    ' One seen= line per item: a,b,c for a snapshot; a,c for a live view; complete if
    ' iteration ends. A caught error prints kind tests (cme/nse/npe/bounds); size=2 afterwards
    ' if mutation ran. b=null after removing the key.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   seen=a
    '   seen=c
    '   cme=false
    '   nse=false
    '   npe=false
    '   bounds=true
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
