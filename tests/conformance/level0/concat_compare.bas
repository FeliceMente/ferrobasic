Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: a comparison after &.
    Dim r As String = "a" & "b" = "ab"
    Log(r)
End Sub
