Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Char `=` a Double: converted with BA.ObjectToChar, the first
    ' character of the number's Java text, as an Int is
    ' (char_number_compare)? 0.00001 is "1.0E-5" in Java, but "0.00001" as
    ' B4J prints it (docs/open-questions.md).
    Dim c As Char = "1"
    Dim d As Double = 0.00001
    Log(c = d)
End Sub
