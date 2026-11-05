Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Show(Null)
End Sub

Sub Show (n As Object)
    ' A For Each without `As` over a parameter of type Object: accepted, as
    ' over a local Object (for_each_untyped_object) and a module variable
    ' Object (for_each_untyped_module_object)? FerroBASIC refuses it
    ' (docs/open-questions.md).
    For Each n In Array As Int(5, 6)
    Next
    Log(n)
End Sub
