Sub Process_Globals
    Dim g As Object = 5
End Sub

Sub AppStart (Args() As String)
    ' A Dim in a Sub with a module variable's name and type resets it
    ' (local_dim_resets_module_variable). For an Object with no value: to a
    ' new Object, as a local Dim gives (object_dim_default), or to Null?
    ' FerroBASIC resets it to Null meanwhile (docs/open-questions.md).
    Reset
    Log(g = Null)
    Try
        Log(GetType(g))
    Catch
        Log("no type")
    End Try
End Sub

Sub Reset
    Dim g As Object
End Sub
