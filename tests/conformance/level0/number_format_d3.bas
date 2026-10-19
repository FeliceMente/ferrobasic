Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' 28551757570297602048, exactly: a double that Java's NumberFormat prints
    ' with 17 significant digits, 28,551,757,570,297,602,000, although 15
    ' round-trip. FerroBASIC prints 28,551,757,570,297,600,000 (deviation D3).
    Dim x As Double = 7137939392574400512 * 4.0
    Log(x)
    Log(NumberFormat(x, 0, 0))
End Sub
