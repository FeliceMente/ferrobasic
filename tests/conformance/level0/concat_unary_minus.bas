Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: a negative number after &.
    Dim r As String = "x" & -1
    Log(r)
End Sub
