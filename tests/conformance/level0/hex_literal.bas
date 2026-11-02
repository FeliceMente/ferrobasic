Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is a hex literal up to 0xFFFFFFFF an Int, and a decimal literal that
    ' does not fit an Int a Long (docs/open-questions.md)?
    Log(0xFF000000)
    Log(0x7FFFFFFF + 1)
    Dim o As Object = 0xFF000000
    Log(GetType(o))
    o = 2147483648
    Log(GetType(o))
End Sub
