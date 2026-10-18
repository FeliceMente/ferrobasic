Sub Process_Globals
    Private Prices As Map
End Sub

Sub AppStart (Args() As String)
    Prices.Initialize
    Prices.Put("coffee", 1.2)
    Prices.Put("croissant", 1.5)
    Dim Total As Double = 0
    For Each K As String In Prices.Keys
        Total = Total + Prices.Get(K)
    Next
    If Total > 2 Then Log("Big breakfast")
    Log($"Total: ${Total}"$)
End Sub
