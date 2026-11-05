Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' < on Booleans: a compile error (docs/open-questions.md)?
    Log(False < True)
End Sub
