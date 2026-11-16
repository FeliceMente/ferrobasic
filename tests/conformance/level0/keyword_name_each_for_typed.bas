Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 23 confirms that B4J refuses this typed counted loop.
    ' At Log(Each) in the body it reports that each was used before it
    ' was assigned a value. The exact diagnostic is in the recording
    ' metadata. FerroBASIC rejects the header because it parses For Each
    ' as the start of a collection loop.
    For Each As Int = 1 To 2
        Log(Each)
    Next
    Log(Each)
End Sub
