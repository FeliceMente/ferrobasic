Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` over a variable declared with Dim. Found by
    ' ferro-compare: FerroBASIC accepts it, and no recording settles it.
    Dim n As Int
    For Each n In Array As Int(5, 6)
        Log(n)
    Next
    Log(n)
End Sub
