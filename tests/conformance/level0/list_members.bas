Sub Process_Globals
    Type Person (Name As String, Age As Int)
End Sub

Sub AppStart (Args() As String)
    Dim l As List
    l.Initialize
    l.AddAll(Array As String("b", "d"))
    l.InsertAt(0, "a")
    l.InsertAt(2, "c")
    l.Add("e")
    Log(Join(l) & " " & l.Size)
    l.RemoveAt(1)
    l.Set(0, "A")
    Log(Join(l))
    Log(l.IndexOf("d") & " " & l.IndexOf("z"))
    ' IndexOf compares with equals: a boxed Int is not a boxed Double.
    Dim i As Int = 1
    Dim d As Double = 1
    Dim nums As List
    nums.Initialize
    nums.Add(i)
    Log(nums.IndexOf(i) & " " & nums.IndexOf(d))
    ' Sort: UTF-16 order, upper case first; stable.
    Dim words As List
    words.Initialize
    words.AddAll(Array As String("pear", "Apple", "banana", "apple", "Cherry"))
    words.Sort(True)
    Log(Join(words))
    words.Sort(False)
    Log(Join(words))
    ' Equal ignoring case keep their current order.
    words.SortCaseInsensitive(True)
    Log(Join(words))
    words.SortCaseInsensitive(False)
    Log(Join(words))
    ' Numbers sort by value, or as text when case-insensitive.
    Dim values As List
    values.Initialize
    values.AddAll(Array As Int(10, 2, 33, 4))
    values.Sort(True)
    Log(Join(values))
    values.SortCaseInsensitive(True)
    Log(Join(values))
    ' Records sort by a field, stably.
    Dim people As List
    people.Initialize
    people.Add(NewPerson("bob", 30))
    people.Add(NewPerson("Ann", 25))
    people.Add(NewPerson("carl", 30))
    people.Add(NewPerson("Dora", 25))
    people.SortType("Age", True)
    Log(Names(people))
    people.SortType("Name", False)
    Log(Names(people))
    people.SortTypeCaseInsensitive("Name", True)
    Log(Names(people))
    Try
        Log(l.Get(10))
    Catch
        Log("get out of range")
    End Try
    Try
        l.InsertAt(9, "x")
    Catch
        Log("insert out of range")
    End Try
    l.Clear
    Log(l.Size)
End Sub

Sub NewPerson (Name As String, Age As Int) As Person
    Dim p As Person
    p.Initialize
    p.Name = Name
    p.Age = Age
    Return p
End Sub

Sub Join (Items As List) As String
    Dim s As String = ""
    For Each x As Object In Items
        If s.Length > 0 Then s = s & ","
        s = s & x
    Next
    Return s
End Sub

Sub Names (People As List) As String
    Dim s As String = ""
    For Each p As Person In People
        s = s & p.Name & p.Age & " "
    Next
    Return s.Trim
End Sub
