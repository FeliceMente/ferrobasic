Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A String compared with a number: does the left operand decide, so it
    ' compares as Strings (docs/open-questions.md)?
    Log("10" < 9)
    Log("10.0" = 10)
End Sub
