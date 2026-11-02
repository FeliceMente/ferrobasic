Sub Process_Globals
    Type Point (X As Int)
    ' A loop variable of Process_Globals named like a Type
    ' (docs/open-questions.md).
    For Point = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Log("built")
End Sub
