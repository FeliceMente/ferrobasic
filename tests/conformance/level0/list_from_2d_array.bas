Sub AppStart (Args() As String)
    Dim Grid(2, 3) As Int
    Grid(0, 1) = 5
    Dim Rows As List
    Rows.Initialize2(Grid)
    Log(Rows.Size)
    Dim First() As Int = Rows.Get(0)
    Log(First.Length & " " & First(1))
    First(2) = 7
    Log(Grid(0, 2))
    Rows.Set(1, Array As Int(1, 2))
    Log(Grid(1, 1))
    Try
        Log(Grid(1, 2))
    Catch
        Log("short row")
    End Try
    Try
        Rows.Set(0, "x")
    Catch
        Log("not a row")
    End Try
    Rows.Set(0, Null)
    Try
        Log(Grid(0, 0))
    Catch
        Log("null row")
    End Try
End Sub
