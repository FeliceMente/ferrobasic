Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: a multiplication inside a division after &.
    Dim r As String = "x" & 2 * 6 / 3
    Log(r)
End Sub
