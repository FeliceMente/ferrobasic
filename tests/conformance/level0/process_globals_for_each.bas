Sub Process_Globals
    Dim Total As Int
    ' May For Each declare its variable in Process_Globals? FerroBASIC
    ' rejects it until a recording shows.
    For Each n As Int In Array As Int(1, 2, 3)
        Total = Total + n
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Total)
End Sub
