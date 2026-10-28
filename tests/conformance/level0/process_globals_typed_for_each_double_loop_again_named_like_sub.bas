Sub Process_Globals
    ' A For Each As Double again in Process_Globals over a name like a
    ' Sub's: accepted, as with As Int? FerroBASIC rejects it until a
    ' recording shows.
    For Each Bump As Double In Array As Double(1.5)
    Next
    For Each Bump As Double In Array As Double(2.5)
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
