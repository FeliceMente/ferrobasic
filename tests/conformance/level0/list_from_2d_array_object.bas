Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A 2-dimensional array reaches List.Initialize2 through an Object, which
    ' the compiler cannot refuse (docs/open-questions.md).
    Dim Grid(2, 3) As Int
    Dim O As Object = Grid
    Dim Rows As List
    Try
        Rows.Initialize2(O)
        Log(Rows.Size)
        Dim First() As Int = Rows.Get(0)
        Log(First.Length)
    Catch
        Log(LastException.Message.Contains("ClassCast"))
    End Try
End Sub
