Sub Process_Globals
    ' A module variable Object declared with no value: a new Object, as a
    ' local Dim gives (object_dim_default), recording run 18.
    Private g As Object
End Sub

Sub AppStart (Args() As String)
    Log(g = Null)
    Try
        Log(GetType(g))
    Catch
        Log("no type")
    End Try
End Sub
