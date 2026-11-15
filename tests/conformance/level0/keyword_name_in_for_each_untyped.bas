Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept In as the variable of an untyped For Each
    ' (docs/open-questions.md), where it stands right before the delimiter
    ' In? It is recorded only as a parameter (contextual_in), and
    ' keyword_name_in_for_each asks the typed form. If it builds, this
    ' prints 3 and 4; otherwise B4J refuses it. FerroBASIC accepts it
    ' meanwhile.
    For Each In In Array As Int(3, 4)
        Log(In)
    Next
End Sub
