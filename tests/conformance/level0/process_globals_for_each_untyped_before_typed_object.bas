Sub Process_Globals
    ' A For Each without `As` in Process_Globals, then a typed loop there
    ' that declares the same name an Object: accepted, as with a Dim after
    ' it (process_globals_for_each_untyped_before_dim_object) or the typed
    ' loop first (process_globals_for_each_untyped_after_typed_object):
    ' recording run 20.
    For Each n In Array As Int(5, 6)
    Next
    For Each n As Object In Array As Int(7)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(n)
End Sub
