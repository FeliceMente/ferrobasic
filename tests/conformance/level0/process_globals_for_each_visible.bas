Sub Process_Globals
    Dim Total As Int
    ' Is a For Each variable of Process_Globals a module variable, seen by
    ' AppStart, as an undeclared For counter there is? FerroBASIC rejects it
    ' until a recording shows.
    For Each n As Int In Array As Int(1, 2, 3)
        Total = Total + n
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Total)
    Log(n)
End Sub
