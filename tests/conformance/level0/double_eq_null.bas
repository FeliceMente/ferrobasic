Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Double `=` Null: a NumberFormatException when it runs, as with an Int
    ' (null_number_compare)? (docs/open-questions.md)
    Dim d As Double = 0
    Log(d = Null)
End Sub
