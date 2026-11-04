Sub Process_Globals
    ' A For Each without `As` in Process_Globals, then a Dim there of the
    ' same name as an Object: accepted, as with the Dim first
    ' (process_globals_for_each_untyped_object)? FerroBASIC refuses it
    ' (docs/open-questions.md).
    For Each n In Array As Int(5, 6)
    Next
    Dim n As Object
End Sub

Sub AppStart (Args() As String)
    Log(GetType(n))
End Sub
