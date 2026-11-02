Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Are a For loop's limit and step evaluated once
    ' (docs/open-questions.md)?
    Dim Limit As Int = 3
    Dim Stride As Int = 1
    For i = 1 To Limit Step Stride
        Log(i)
        Limit = 10
        Stride = 2
    Next
End Sub
