Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' How a variable of type Exception prints (docs/open-questions.md).
    Try
        Dim l As List
        l.Initialize
        Log(l.Get(3))
    Catch
        Dim e As Exception = LastException
        Log(e.IsInitialized)
        Log(e)
    End Try
    Dim Unset As Exception
    Log(Unset)
End Sub
