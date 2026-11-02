Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A number compared with a String: does the left operand decide, so it
    ' compares numerically (docs/open-questions.md)?
    Log(10 < "9")
    Log(10 = "10.0")
End Sub
