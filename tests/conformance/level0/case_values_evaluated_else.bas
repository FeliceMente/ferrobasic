Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Every Case value is evaluated before matching
    ' (case_values_evaluated): also when none matches and Case Else runs
    ' (docs/open-questions.md)?
    Select 9
        Case Value(1)
            Log("one")
        Case Value(2), Value(3)
            Log("two or three")
        Case Else
            Log("else")
    End Select
End Sub

Sub Value (N As Int) As Int
    Log(N)
    Return N
End Sub
