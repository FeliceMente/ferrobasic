Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Rnd is Min + nextInt(Max - Min) on java.util.Random, so a seeded
    ' sequence is the same everywhere.
    RndSeed(42)
    Log(Draw)
    RndSeed(42)
    Log(Draw)
    RndSeed(-7)
    Log(Rnd(-5, 5) & " " & Rnd(-5, 5) & " " & Rnd(0, 1) & " " & Rnd(1000000, 2000000))
    Try
        Log(Rnd(5, 5))
    Catch
        Log("empty range")
    End Try
End Sub

Sub Draw As String
    Dim s As String = ""
    For i = 1 To 8
        s = s & Rnd(0, 100) & " "
    Next
    Return s.Trim
End Sub
