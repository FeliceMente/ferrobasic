Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: division after &.
    Dim r As String = "x" & 6 / 2
    Log(r)
End Sub
