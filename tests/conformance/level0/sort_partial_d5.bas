Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Java's sort stops at the first pair that does not compare (3 against
    ' "x"), after sorting 1, 3, 2 in place; FerroBASIC checks first.
    Dim L As List
    L.Initialize
    L.AddAll(Array As Object(1, 3, 2, "x"))
    Try
        L.Sort(True)
    Catch
        Log(LastException.Message.Contains("ClassCast"))
    End Try
    Log(L)
End Sub
