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
    ' A row can be any array its type admits: here an Int(,) in an Object(,).
    Dim Things(2, 2) As Object
    Dim ThingRows As List
    ThingRows.Initialize2(Things)
    Dim Inner(1, 1) As Int
    ThingRows.Set(0, Inner)
    Try
        Things(0, 0) = 7
    Catch
        Log("an Int row holds arrays")
    End Try
    Dim Cells(1) As Int
    Things(0, 0) = Cells
    Log("stored a row")
End Sub
