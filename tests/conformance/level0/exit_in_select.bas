Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim count As Int = 0
    For i = 1 To 5
        Select i
            Case 3
                Exit
                Log("not reached")
        End Select
        count = count + 1
    Next
    Log(count)
End Sub
