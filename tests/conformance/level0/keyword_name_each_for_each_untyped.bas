Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept Each as the variable of an untyped For Each
    ' (docs/open-questions.md)? It is recorded only as a local variable
    ' (keyword_name_each), and keyword_name_each_for_each asks the typed
    ' form. If it builds, this prints 3 and 4; otherwise B4J refuses it.
    ' FerroBASIC accepts it meanwhile.
    For Each Each In Array As Int(3, 4)
        Log(Each)
    Next
End Sub
