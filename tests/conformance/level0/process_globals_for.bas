Sub Process_Globals
    Dim Total As Int
    ' An implicit loop variable in Process_Globals: a module variable, a
    ' local, or an error? FerroBASIC rejects it until a recording shows.
    For i = 1 To 3
        Total = Total + i
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Total)
End Sub
