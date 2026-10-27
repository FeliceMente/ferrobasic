Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each As inside a For over the same name like a Sub's: does it
    ' share the counter, or have its own variable, or is it refused?
    ' FerroBASIC rejects it until a recording shows.
    Dim n As Int
    For Bump = 1 To 3
        n = n + 1
        For Each Bump As Int In Array As Int(7, 8)
            Log(Bump)
        Next
        If n > 5 Then Exit
    Next
    Log(n)
End Sub

Sub Bump As Int
    Return 9
End Sub
