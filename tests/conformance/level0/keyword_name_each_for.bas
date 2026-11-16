Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 22 confirms that B4J refuses this counted loop.
    ' At Log(Each) it reports that each was used before it was assigned
    ' a value. The exact diagnostic is in the recording metadata.
    ' FerroBASIC rejects the header because it parses For Each as
    ' the start of a collection loop.
    For Each = 1 To 2
        Log(Each)
    Next
End Sub
