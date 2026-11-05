Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Comparing a Char with a String: as Strings (docs/open-questions.md)?
    Dim c As Char = "A"
    Log(c = "A")
    Log(c < "B")
End Sub
