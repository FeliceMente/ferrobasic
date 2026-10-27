Sub Process_Globals
    Type A (N As Int)
End Sub

Sub AppStart (Args() As String)
    ' The lines of list_sort_type_class_fields that FerroBASIC matches: a
    ' Type's class has IsInitialized, and a name no class declares raises.
    ' Its third line, a JDK class's private field, is deviation D7.
    Dim U, I As A
    I.Initialize
    Dim L As List
    L.Initialize
    L.Add(I)
    L.Add(U)
    Log(Sorted(L, "IsInitialized"))
    Log(L.Get(0) = U)
    Dim Numbers As List
    Numbers.Initialize
    Numbers.AddAll(Array As Int(2, 1))
    Log(Sorted(Numbers, "missing"))
End Sub

' "sorted", or the error's kind without the JDK's message text.
Sub Sorted (L As List, Field As String) As String
    Try
        L.SortType(Field, True)
    Catch
        If LastException.Message.Contains("NoSuchField") Then Return "NoSuchField"
        Return "other"
    End Try
    Return "sorted"
End Sub
