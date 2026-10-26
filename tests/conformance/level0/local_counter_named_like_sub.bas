Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' An undeclared For counter of a Sub with a Sub's name: a local (this
    ' prints 1, then 2), the Sub (9, 9), or refused as a Dim is? FerroBASIC
    ' rejects it until a recording shows.
    For Bump = 1 To 2
        Log(Bump)
    Next
End Sub

Sub Bump As Int
    Return 9
End Sub
