Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A number `=` a Char, the number on the left: B4J converts the number
    ' to a Char when the Char is on the left (char_number_compare); does it
    ' convert the Char to a number here (docs/open-questions.md)?
    Dim c As Char = "6"
    Log(65 = c)
End Sub
