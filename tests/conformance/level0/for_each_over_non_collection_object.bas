Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A For Each over an Object that holds a number, not a collection: B4J
    ' raises what (docs/open-questions.md)? FerroBASIC stops with an
    ' internal error meanwhile.
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
