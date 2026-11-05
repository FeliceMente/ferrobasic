Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Assigning a Double to an Int: truncated toward zero, or rounded
    ' (docs/open-questions.md)?
    Dim i As Int
    Dim d As Double = 2.7
    i = d
    Log(i)
    d = -2.7
    i = d
    Log(i)
    d = 2.5
    i = d
    Log(i)
    d = 3.5
    i = d
    Log(i)
    d = -2.5
    i = d
    Log(i)
End Sub
