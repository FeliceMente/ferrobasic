Sub Process_Globals
    Type A (N As Object)
    Type B (N As Object)
End Sub

Sub AppStart (Args() As String)
    ' One item: nothing is compared.
    Show("one", Sorted(One(NewA(1)), False))
    ' Items of another type, or not records.
    Show("A, B", Sorted(Two(NewA(2), NewB(1)), False))
    Show("A, B case-insensitive", Sorted(Two(NewA(2), NewB(1)), True))
    Show("A, 5", Sorted(Two(NewA(2), 5), False))
    Show("A, Null", Sorted(Two(NewA(2), Null), False))
    ' Field values that cannot be compared.
    Show("Null field", Sorted(Two(NewA(Null), NewA(1)), False))
    Show("Null field case-insensitive", Sorted(Two(NewA(Null), NewA(1)), True))
    Show("Int and String fields", Sorted(Two(NewA(1), NewA("x")), False))
    Dim L1, L2 As List
    L1.Initialize
    L2.Initialize
    Show("List fields", Sorted(Two(NewA(L1), NewA(L2)), False))
    ' A Null first item has no class to take the field from.
    Show("Null first", Sorted(Two(Null, NewA(1)), False))
End Sub

Sub One (First As Object) As List
    Dim L As List
    L.Initialize
    L.Add(First)
    Return L
End Sub

Sub Two (First As Object, Second As Object) As List
    Dim L As List = One(First)
    L.Add(Second)
    Return L
End Sub

' Sorts by N and returns the N values in order, or the error's kind.
Sub Sorted (L As List, IgnoreCase As Boolean) As String
    Try
        If IgnoreCase Then
            L.SortTypeCaseInsensitive("N", True)
        Else
            L.SortType("N", True)
        End If
    Catch
        Return Kind(LastException.Message)
    End Try
    Dim Out As StringBuilder
    Out.Initialize
    For Each Item As Object In L
        Dim T As A = Item
        Out.Append(T.N).Append(" ")
    Next
    Return "sorted " & Out.ToString.Trim
End Sub

Sub Show (Name As String, Result As String)
    Log(Name & ": " & Result)
End Sub

Sub NewA (N As Object) As A
    Dim T As A
    T.Initialize
    T.N = N
    Return T
End Sub

Sub NewB (N As Object) As B
    Dim T As B
    T.Initialize
    T.N = N
    Return T
End Sub

' The exception's kind, and the kind it wraps, without the JDK's message text.
Sub Kind (Message As String) As String
    Dim Inner As String = ""
    For Each Name As String In Array As String("IllegalArgument", "NullPointer", "ClassCast")
        If Message.Contains(Name) Then Inner = Name
    Next
    If Message.StartsWith("java.lang.RuntimeException") Then Return "RuntimeException(" & Inner & ")"
    Return Inner
End Sub
