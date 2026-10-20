Sub Process_Globals
    Private Grid(2, 2) As Int
    Private Rows As List
End Sub

Sub AppStart (Args() As String)
    Rows.Initialize2(Grid)
    ' Java selects the row before it evaluates the value assigned...
    Dim OldFirst() As Int = Rows.Get(0)
    Grid(0, 0) = ReplaceFirstRow
    Log(OldFirst(0) & " " & Grid(0, 0))
    ' ...and before it evaluates the next index.
    Dim OldSecond() As Int = Rows.Get(1)
    OldSecond(1) = 5
    Log(Grid(1, ReplaceSecondRow))
    ' A bad row index fails before the next index is evaluated.
    Try
        Log(Grid(5, Noisy))
    Catch
        Log("bad row")
    End Try
End Sub

Sub ReplaceFirstRow As Int
    Rows.Set(0, Array As Int(10, 20))
    Return 7
End Sub

Sub ReplaceSecondRow As Int
    Rows.Set(1, Array As Int(30, 40))
    Return 1
End Sub

Sub Noisy As Int
    Log("evaluated")
    Return 0
End Sub
