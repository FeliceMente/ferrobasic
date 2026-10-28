Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Three For Each As Int nested over a name like a Sub's: accepted, as
    ' two are? FerroBASIC rejects it until a recording shows.
    For Each Bump As Int In Array As Int(1)
        For Each Bump As Int In Array As Int(2)
            For Each Bump As Int In Array As Int(5)
                Log(Bump)
            Next
        Next
    Next
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
