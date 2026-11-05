Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' `Return <value>` in a Sub declared without `As` (docs/open-questions.md).
    NoType
    Log("called")
End Sub

Sub NoType
    Return 5
End Sub
