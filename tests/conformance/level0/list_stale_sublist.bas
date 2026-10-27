Sub Process_Globals
    Type Item (N As Int)
End Sub

Sub AppStart (Args() As String)
    ' Changes through a view whose parent changed: the view refuses first.
    Dim V As List = Stale
    Try
        V.Add(9)
    Catch
        Log("Add: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        V.InsertAt(0, 9)
    Catch
        Log("InsertAt: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        V.RemoveAt(0)
    Catch
        Log("RemoveAt: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        V.Set(0, 9)
    Catch
        Log("Set: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        V.Clear
    Catch
        Log("Clear: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        V.Sort(True)
    Catch
        Log("Sort: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        V.SortCaseInsensitive(True)
    Catch
        Log("SortCaseInsensitive: " & Kind(LastException.Message))
    End Try
    ' SortType reads the size first, which the stale view refuses.
    V = Stale
    Try
        V.SortType("N", True)
    Catch
        Log("SortType: " & Kind(LastException.Message))
    End Try
    V = Stale
    Try
        Log(V.Size)
    Catch
        Log("Size: " & Kind(LastException.Message))
    End Try

    ' On a live view, SortType looks up the field on the first item before
    ' the view refuses to sort.
    Dim Items As List
    Items.Initialize
    Items.Add(NewItem(2))
    Items.Add(NewItem(1))
    Dim Live As List = Items.SubList(0, 2)
    Try
        Live.SortType("N", True)
    Catch
        Log("SortType N: " & Kind(LastException.Message))
    End Try
    Try
        Live.SortType("Missing", True)
    Catch
        Log("SortType Missing: " & Kind(LastException.Message))
    End Try
End Sub

' A view of [3, 1] whose parent then grew.
Sub Stale As List
    Dim Parent As List
    Parent.Initialize
    Parent.AddAll(Array As Int(3, 1, 2))
    Dim View As List = Parent.SubList(0, 2)
    Parent.Add(4)
    Return View
End Sub

Sub NewItem (N As Int) As Item
    Dim T As Item
    T.Initialize
    T.N = N
    Return T
End Sub

' The exception's kind, without the JDK's message text.
Sub Kind (Message As String) As String
    For Each Name As String In Array As String("UnsupportedOperation", "ConcurrentModification", "NoSuchField")
        If Message.Contains(Name) Then Return Name
    Next
    Return Message
End Sub
