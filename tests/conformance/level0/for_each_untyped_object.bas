Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` over a local Object: refused as over an Int or
    ' a String (for_each_untyped_local), or accepted, as an Object takes any
    ' item (docs/open-questions.md)?
    Dim o As Object
    For Each o In Array As Int(5, 6)
        Log(o)
    Next
End Sub
