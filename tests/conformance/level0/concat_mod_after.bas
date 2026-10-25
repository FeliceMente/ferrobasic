Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: Mod after &.
    Dim r As String = "x" & 7 Mod 2
    Log(r)
End Sub
