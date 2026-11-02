Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The value of a For counter after the loop, after it ran and when it
    ' never ran (docs/open-questions.md).
    Dim i As Int
    For i = 1 To 3
    Next
    Log(i)
    For i = 5 To 1
    Next
    Log(i)
    For i = 10 To 1 Step -3
    Next
    Log(i)
End Sub
