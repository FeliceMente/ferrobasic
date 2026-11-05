Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' An undeclared variable that is not a For counter: an implicit String,
    ' or a compile error (docs/open-questions.md)?
    Undeclared = 5
    Log(Undeclared)
End Sub
