Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A typed For counter named like a Sub: accepted, with Bump the Sub
    ' inside the loop, as for an undeclared counter? FerroBASIC rejects it
    ' until a recording shows.
    For Bump As Int = 1 To 2
        Log(Bump)
    Next
End Sub

Sub Bump As Int
    Return 9
End Sub
