Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Char `=` a Float: the first character of the Float's Java text, as
    ' with a Double (char_eq_double)? FerroBASIC refuses it
    ' (docs/open-questions.md).
    Dim c As Char = "1"
    Dim f As Float = 0.00001
    Log(c = f)
End Sub
