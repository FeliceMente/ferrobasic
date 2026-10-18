Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' In is a keyword in For Each and may also name a parameter.
    Dim Total As Int = 0
    For Each n As Int In Array As Int(1, 2, 3)
        Total = Total + Twice(n)
    Next
    Log(Total)
End Sub

Sub Twice (In As Int) As Int
    Return Add(In, In)
End Sub

Sub Add (a As Int, b As Int) As Int
    Return a + b
End Sub
