Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each without `As` whose variable is a String, over Ints: converted,
    ' or an error? Found by ferro-compare: FerroBASIC accepts it.
    Dim s As String
    For Each s In Array As Int(5, 6)
        Log(s & "!")
    Next
End Sub
