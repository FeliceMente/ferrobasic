Sub Process_Globals
    Type A (N As Int)
End Sub

Sub AppStart (Args() As String)
    ' The field is looked up on the first item's Java class, so it may be
    ' one the program does not declare: a Type's IsInitialized, or a field
    ' of a JDK class.
    Dim U, I As A
    I.Initialize
    Dim L As List
    L.Initialize
    L.Add(I)
    L.Add(U)
    Log(Sorted(L, "IsInitialized"))
    Log(L.Get(0) = U)
    Log(Sorted(Numbers, "value"))
    Log(Sorted(Numbers, "missing"))
End Sub

Sub Numbers As List
    Dim L As List
    L.Initialize
    L.AddAll(Array As Int(2, 1))
    Return L
End Sub

' "sorted", or the error's kind without the JDK's message text.
Sub Sorted (L As List, Field As String) As String
    Try
        L.SortType(Field, True)
    Catch
        For Each Name As String In Array As String("NoSuchField", "InaccessibleObject", "IllegalAccess")
            If LastException.Message.Contains(Name) Then Return Name
        Next
        Return "other"
    End Try
    Return "sorted"
End Sub
