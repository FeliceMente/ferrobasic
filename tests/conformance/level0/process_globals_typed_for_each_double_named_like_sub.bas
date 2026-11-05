Sub Process_Globals
    ' A For Each of another type than Int, in Process_Globals, over a name
    ' of a Sub: only Int is recorded there. Found by ferro-compare:
    ' FerroBASIC accepts it.
    For Each Bump As Double In Array As Double(1.5)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Bump)
End Sub

Sub Bump As Int
    Return 7
End Sub
