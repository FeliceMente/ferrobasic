Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A one-dimensional array reaches List.Initialize2 through an Object
    ' (docs/open-questions.md): a 2-dimensional one raises ClassCastException.
    Dim Numbers() As Int = Array As Int(1, 2, 3)
    Dim O As Object = Numbers
    Dim L As List
    Try
        L.Initialize2(O)
        Log(L.Size)
    Catch
        Log(LastException.Message.Contains("ClassCast"))
    End Try
End Sub
