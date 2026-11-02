Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' In which order a For evaluates its start, limit and step
    ' (docs/open-questions.md)?
    For i = Value("start", 1) To Value("limit", 2) Step Value("step", 1)
        Log(i)
    Next
End Sub

Sub Value (Name As String, N As Int) As Int
    Log(Name)
    Return N
End Sub
