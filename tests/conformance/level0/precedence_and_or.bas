Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Relative precedence of And and Or (docs/open-questions.md).
    Log(True Or False And False)
    Log(False And True Or True)
End Sub
