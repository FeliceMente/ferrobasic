Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' `<` between a String and an Object: B4J parses a String for `<`
    ' (string_compare_less); does it convert the Object to a number too
    ' (docs/open-questions.md)?
    Dim o As Object = "5"
    Log("10" < o)
    Log(o < "10")
End Sub
