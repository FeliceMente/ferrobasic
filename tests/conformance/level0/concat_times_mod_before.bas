Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: a multiplication inside a Mod before &.
    Dim r As String = 2 * 3 Mod 4 & "x"
    Log(r)
End Sub
