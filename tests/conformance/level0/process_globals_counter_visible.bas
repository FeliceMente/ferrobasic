Sub Process_Globals
    Dim Total As Int
    ' Is an undeclared counter in Process_Globals a module variable, seen by
    ' AppStart? FerroBASIC makes it a local of Process_Globals, so it rejects
    ' this until a recording shows.
    For i = 1 To 3
        Total = Total + i
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Total)
    Log(i)
End Sub
