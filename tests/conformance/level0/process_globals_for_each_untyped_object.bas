Sub Process_Globals
    ' A For Each without `As` in Process_Globals over a variable Object
    ' declared there: accepted, as from a Sub
    ' (for_each_untyped_module_object), recording run 18.
    Private n As Object
    For Each n In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
