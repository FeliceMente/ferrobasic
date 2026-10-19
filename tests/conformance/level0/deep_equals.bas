Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' How deeply nested Lists can be compared before Java's recursive
    ' equals overflows (docs/open-questions.md). Each depth that compares
    ' logs the result; the first one that fails stops the program, so a
    ' recording shows the threshold.
    Dim Depths() As Int = Array As Int(10, 100, 500, 1000, 2000, 5000, 10000, 100000)
    For Each Depth As Int In Depths
        Dim A As List
        A.Initialize
        Dim B As List
        B.Initialize
        For i = 2 To Depth
            Dim NextA As List
            NextA.Initialize
            NextA.Add(A)
            A = NextA
            Dim NextB As List
            NextB.Initialize
            NextB.Add(B)
            B = NextB
        Next
        Dim OA As Object = A
        Dim OB As Object = B
        Log(Depth & ": " & (OA = OB))
    Next
End Sub
