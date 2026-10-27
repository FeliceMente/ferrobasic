Sub Process_Globals
    ' A For Each variable of Process_Globals, then an undeclared For counter
    ' of the same name: one module variable (this prints 3)? FerroBASIC
    ' rejects it until a recording shows.
    For Each n As Int In Array As Int(5, 6)
    Next
    For n = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
