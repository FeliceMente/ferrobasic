Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' `<` between a String and a Boolean: refused, as between two Booleans
    ' ("Cannot cast type: {Type=Boolean,...} to: number", boolean_less)?
    ' (docs/open-questions.md)
    Log("a" < True)
End Sub
