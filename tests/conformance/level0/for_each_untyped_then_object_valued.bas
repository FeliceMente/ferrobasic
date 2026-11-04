Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As`, which declares its variable an Object, then
    ' a Dim of the same name as an Object with a value: one variable, as
    ' with a Dim with no value (for_each_untyped_object_declared_after).
    ' Recording run 18 builds it.
    For Each n In Array As Int(5, 6)
        Log(n)
    Next
    Dim n As Object = 7
    Log(n)
End Sub
