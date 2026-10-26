Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each variable named like a Sub of another type: is Bump.Length
    ' the String's length, or an error on the Sub's Int? FerroBASIC rejects
    ' it until a recording shows.
    For Each Bump As String In Array As String("abc")
        Log(Bump.Length)
    Next
End Sub

Sub Bump As Int
    Return 9
End Sub
