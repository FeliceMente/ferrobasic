Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The value a Sub without `As` returns, read: B4J declares such a Sub
    ' as returning a String, so does Log(NoType) print 5? FerroBASIC gives
    ' it no value (docs/open-questions.md).
    Log(NoType)
End Sub

Sub NoType
    Return 5
End Sub
