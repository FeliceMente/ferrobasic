Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each As Double again over a name like a Sub's: accepted, as with
    ' As Int? FerroBASIC rejects it until a recording shows.
    For Each Bump As Double In Array As Double(1.5)
    Next
    For Each Bump As Double In Array As Double(2.5)
        Log(Bump)
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
