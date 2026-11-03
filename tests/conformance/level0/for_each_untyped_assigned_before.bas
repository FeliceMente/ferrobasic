Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' An assignment to an undeclared name, then a For Each without `As` over
    ' it: the assignment declares a String (undeclared_variable_type), the
    ' loop an Object (for_each_untyped_declared_after), so refused? Or one
    ' variable? FerroBASIC refuses it (docs/open-questions.md).
    u = 5
    Log(u)
    For Each u In Array As Int(7)
    Next
    Log(u)
End Sub
