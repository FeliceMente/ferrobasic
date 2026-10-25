Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: multiplication after &.
    Dim r As String = "x" & 2 * 3
    Log(r)
End Sub
