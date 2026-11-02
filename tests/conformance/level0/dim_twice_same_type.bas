Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Two Dims of the same name and type in one Sub: built, and does the
    ' second reset the variable (docs/open-questions.md)?
    Dim n As Int = 3
    Log(n)
    Dim n As Int
    Log(n)
End Sub
