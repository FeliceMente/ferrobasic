Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The value of a Sub without `As` (untyped_sub_return_value_used prints
    ' 5): a String, as B4J generates `return "";` for such Subs, so ""
    ' when it returns nothing (docs/open-questions.md)?
    Log(GetType(NoType))
    Log(NoType + 1)
    Log("[" & Nothing & "]")
End Sub

Sub NoType
    Return 5
End Sub

Sub Nothing
End Sub
