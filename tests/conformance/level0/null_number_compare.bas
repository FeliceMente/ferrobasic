Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Null compared with a number: a compile error (docs/open-questions.md)?
    Dim n As Int = 0
    Log(n = Null)
End Sub
