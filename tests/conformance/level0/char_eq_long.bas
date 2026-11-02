Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Char `=` a Long, a Short or a Byte: converted with BA.ObjectToChar,
    ' the first character of the number's text, as an Int is
    ' (char_number_compare)? (docs/open-questions.md)
    Dim c As Char = "6"
    Dim l As Long = 65
    Log(c = l)
    Dim s As Short = 65
    Log(c = s)
    Dim b As Byte = 65
    Log(c = b)
End Sub
