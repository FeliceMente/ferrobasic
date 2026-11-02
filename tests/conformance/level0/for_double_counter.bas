Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For over a Double variable, with a fractional step. Found by
    ' ferro-compare: FerroBASIC accepts it, and no recording settles it.
    Dim d As Double
    For d = 1 To 2 Step 0.5
        Log(d)
    Next
    Log(d)
End Sub
