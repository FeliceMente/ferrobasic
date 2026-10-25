Sub Process_Globals
    ' A name that is both an undeclared For counter (an Int) and a For Each
    ' variable of another type in Process_Globals: B4J refuses it.
    For n = 1 To 2
    Next
    For Each n As String In Array As String("a", "b")
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
