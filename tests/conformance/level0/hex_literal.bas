Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is a hex literal up to 0xFFFFFFFF an Int (docs/open-questions.md)?
    ' decimal_literal_over_int asks about decimal literals.
    Log(0xFF000000)
    Log(0x7FFFFFFF + 1)
    Dim o As Object = 0xFF000000
    Log(GetType(o))
End Sub
