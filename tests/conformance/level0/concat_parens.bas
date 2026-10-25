Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: arithmetic in parentheses after &.
    Dim r As String = "x" & (1 + 2)
    Log(r)
End Sub
