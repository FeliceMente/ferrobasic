Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Long `=` Null: a NumberFormatException when it runs, as with an Int
    ' or a Double (null_number_compare, double_eq_null)? FerroBASIC
    ' refuses it (docs/open-questions.md).
    Dim n As Long = 0
    Log(n = Null)
End Sub
