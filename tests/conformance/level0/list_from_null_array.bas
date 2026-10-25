Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A declared array that is Null, passed where a List is expected:
    ' ArrayToList hands it to Arrays.asList, which refuses null.
    Dim Nothing As Object = Null
    Dim Numbers() As Int = Nothing
    Dim L As List
    Try
        L.Initialize2(Numbers)
        Log(L.IsInitialized)
    Catch
        Log(LastException.Message.Contains("NullPointer"))
    End Try
End Sub
