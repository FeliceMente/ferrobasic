Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 21 (B4J 10.70): this fails to build in generated
    ' Java: Object cannot be converted to IterableList. A Catch cannot
    ' handle this compile-time error.
    Dim o As Object = 5
    Try
        For Each v In o
            Log(v)
        Next
        Log("no error")
    Catch
        Log(LastException.Message)
    End Try
End Sub
