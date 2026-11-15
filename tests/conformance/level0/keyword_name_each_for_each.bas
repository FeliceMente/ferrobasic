Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept Each as the variable of a typed For Each
    ' (docs/open-questions.md)? It is recorded only as a local variable
    ' (keyword_name_each). If it builds, this prints 3 and 4; otherwise B4J
    ' refuses it. FerroBASIC accepts it meanwhile.
    For Each Each As Int In Array As Int(3, 4)
        Log(Each)
    Next
End Sub
