Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A limit that reads the counter: B4J evaluates the limit before it
    ' assigns the start (for_evaluation_order), so the limit should be the
    ' counter's value before the loop, 3 (docs/open-questions.md).
    Dim i As Int = 3
    For i = 1 To i
        Log(i)
    Next
End Sub
