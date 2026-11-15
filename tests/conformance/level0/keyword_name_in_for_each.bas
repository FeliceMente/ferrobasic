Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept In as the variable of a For Each (docs/open-
    ' questions.md)? It is recorded only as a parameter (contextual_in). If
    ' it builds, this prints 3 and 4; otherwise B4J refuses it. FerroBASIC
    ' accepts it meanwhile.
    For Each In As Int In Array As Int(3, 4)
        Log(In)
    Next
End Sub
