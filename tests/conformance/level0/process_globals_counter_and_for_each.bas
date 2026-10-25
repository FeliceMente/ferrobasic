Sub Process_Globals
    ' A name that is both an undeclared For counter and a For Each variable
    ' in Process_Globals: does B4X accept it, and with which type?
    ' FerroBASIC rejects it until a recording shows.
    For n = 1 To 2
    Next
    For Each n As String In Array As String("a", "b")
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
