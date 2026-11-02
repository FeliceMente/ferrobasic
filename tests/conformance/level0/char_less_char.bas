Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' `<` between two Chars: refused, as a Char next to a String is
    ' ("Cannot cast type: {Type=Char,...} to: number", char_string_compare),
    ' or does it compare their codes (docs/open-questions.md)?
    Dim a As Char = "A"
    Dim b As Char = "B"
    Log(a < b)
End Sub
