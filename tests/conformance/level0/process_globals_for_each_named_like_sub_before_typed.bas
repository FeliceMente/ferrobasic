Sub Process_Globals
    ' A For Each without As as the first loop of Process_Globals over a
    ' name like a Sub's, which only the typed For Each after it declares:
    ' refused, as after another loop? FerroBASIC rejects it until a
    ' recording shows.
    For Each Bump In Array As Int(5)
        Log(Bump)
    Next
    For Each Bump As Int In Array As Int(6)
        Log(Bump)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("looped")
End Sub

Sub Bump As Int
    Return 9
End Sub
