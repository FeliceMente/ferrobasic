Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Is `Select Case` a synonym of `Select`, or an error
    ' (docs/open-questions.md)?
    Dim n As Int = 2
    Select Case n
        Case 1
            Log("one")
        Case 2
            Log("two")
    End Select
End Sub
