Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Are all Case values evaluated before matching, or in order until one
    ' matches (docs/open-questions.md)?
    Select 1
        Case Value(1), Value(2)
            Log("matched")
        Case Value(3)
            Log("third")
    End Select
End Sub

Sub Value (N As Int) As Int
    Log(N)
    Return N
End Sub
