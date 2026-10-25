Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: arithmetic before &.
    Dim r As String = 1 + 2 & "x"
    Log(r)
End Sub
