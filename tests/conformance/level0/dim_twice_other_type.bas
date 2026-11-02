Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Two Dims of the same name with different types in one Sub
    ' (docs/open-questions.md)?
    Dim n As Int = 3
    Log(n)
    Dim n As String = "a"
    Log(n)
End Sub
