Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Precedence of & against + and comparisons, and of And against Or,
    ' is not documented: no provisional output until this is recorded.
    Dim r As String = "x" & 1 + 2
    Log(r)
    Log(True Or False And False)
    Log(False And True Or True)
End Sub
