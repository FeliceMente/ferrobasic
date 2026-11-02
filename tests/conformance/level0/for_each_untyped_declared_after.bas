Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` over a name a Dim declares only after the
    ' loop: refused as over a local declared before it
    ' (for_each_untyped_local), or does the loop come first
    ' (docs/open-questions.md)?
    For Each n In Array As Int(5, 6)
        Log(n)
    Next
    Dim n As Int
    Log(n)
End Sub
