Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Counted loop with Step and Select
    For i = 10 To 0 Step -5
        Select i
            Case 10
                Log("ten")
            Case 5
                Log("five")
            Case Else
                Log("other: " & i)
        End Select
    Next

    ' Do While with Exit
    Dim n As Int = 0
    Do While n < 100
        n = n + 7
        If n Mod 3 = 0 Then Exit
    Loop
    Log("n = " & n)

    ' Try / Catch around a runtime error
    Dim L As List
    Try
        L.Add(1)
    Catch
        Log("caught uninitialized list")
    End Try

    Log(Square(12))
End Sub

Sub Square (x As Int) As Int
    Return x * x
End Sub
