Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is a decimal literal that does not fit an Int a Long
    ' (docs/open-questions.md)?
    Dim o As Object = 2147483648
    Log(GetType(o))
    Dim l As Long = 3000000000
    Log(l + 1)
End Sub
