Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each variable of a Sub with a Sub's name: a local (this prints
    ' 1), the Sub (9), or refused as a Dim is? FerroBASIC rejects it until a
    ' recording shows.
    For Each Bump As Int In Array As Int(1)
        Log(Bump)
    Next
End Sub

Sub Bump As Int
    Return 9
End Sub
