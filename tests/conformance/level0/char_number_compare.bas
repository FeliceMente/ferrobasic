Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Comparing a Char with a number: a compile error (docs/open-questions.md)?
    Dim c As Char = "A"
    Log(c = 65)
End Sub
