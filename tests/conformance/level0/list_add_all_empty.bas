Sub AppStart (Args() As String)
    Dim Empty As List
    Empty.Initialize
    ' A list made from an array has a fixed size: adding nothing is allowed.
    Dim Fixed As List
    Fixed.Initialize2(Array As Int(1, 2))
    Fixed.AddAll(Empty)
    Fixed.AddAllAt(2, Empty)
    Log(Fixed.Size)
    Try
        Fixed.AddAllAt(3, Empty)
    Catch
        Log("index checked first")
    End Try
    Try
        Fixed.AddAll(Array As Int(3))
    Catch
        Log("cannot grow")
    End Try
    ' A read-only view refuses even an empty addition.
    Dim View As List = Fixed.SubList(0, 1)
    Try
        View.AddAll(Empty)
    Catch
        Log("read-only")
    End Try
    ' An empty addition still counts as a change of a growable list.
    Dim Growable As List
    Growable.Initialize
    Growable.AddAll(Array As Int(1, 2, 3))
    Dim Front As List = Growable.SubList(0, 2)
    Growable.AddAll(Empty)
    Try
        Log(Front.Get(0))
    Catch
        Log("view invalidated")
    End Try
End Sub
