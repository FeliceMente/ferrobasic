Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 23 confirms that declaring Each first does not help:
    ' B4J refuses the header with a syntax error at the = sign.
    ' The exact diagnostic is in the recording metadata.
    ' FerroBASIC rejects the header because it parses For Each as
    ' the start of a collection loop.
    Dim Each As Int = 9
    For Each = 1 To 2
        Log(Each)
    Next
    Log(Each)
End Sub
