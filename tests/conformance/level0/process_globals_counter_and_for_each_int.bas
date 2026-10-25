Sub Process_Globals
    ' An undeclared For counter, then a For Each of the same name and type
    ' (Int): one module variable?
    For n = 1 To 2
    Next
    For Each n As Int In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
