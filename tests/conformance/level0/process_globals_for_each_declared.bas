Sub Process_Globals
    Dim n As Int
    ' A For Each that declares, with As, a variable Process_Globals already
    ' declares: is it accepted? FerroBASIC rejects it until a recording shows.
    For Each n As Int In Array As Int(1, 2)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
