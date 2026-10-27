Sub Process_Globals
    ' Two For Each loops of Process_Globals declaring the same name and
    ' type: one module variable (this prints 7)? FerroBASIC rejects it until
    ' a recording shows.
    For Each n As Int In Array As Int(5, 6)
    Next
    For Each n As Int In Array As Int(7)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
