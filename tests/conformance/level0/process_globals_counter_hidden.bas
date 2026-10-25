Sub Process_Globals
    ' The undeclared counter of Process_Globals is a module variable: may a
    ' local of another Sub hide it?
    For i = 1 To 3
    Next
End Sub

Sub AppStart (Args() As String)
    Dim i As String = "s"
    Log(i)
    Show
End Sub

Sub Show
    Log(i)
End Sub
