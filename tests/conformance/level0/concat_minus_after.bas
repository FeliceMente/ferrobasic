Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: subtraction after &.
    Dim r As String = "x" & 3 - 1
    Log(r)
End Sub
