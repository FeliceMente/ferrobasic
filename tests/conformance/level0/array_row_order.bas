Sub Process_Globals
    Private Grid(2, 2) As Int
End Sub

Sub AppStart (Args() As String)
    ' The order in which the indices and the assigned value are evaluated
    ' (docs/open-questions.md).
    Grid(Noisy("row", 1), Noisy("column", 0)) = Noisy("value", 7)
    Log(Grid(1, 0))
    ' A bad row index: does it fail before the next index is evaluated?
    Try
        Log(Grid(Noisy("bad row", 5), Noisy("column", 0)))
    Catch
        Log("caught")
    End Try
End Sub

Sub Noisy (What As String, Value As Int) As Int
    Log(What)
    Return Value
End Sub
