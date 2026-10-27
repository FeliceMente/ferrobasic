Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Where & may stand next to other operators: arithmetic after & (B4J 10.70 refused this one).
    Dim r As String = "x" & 1 + 2
    Log(r)
End Sub
