Sub Process_Globals
    Type P (o As Object, n As Int)
End Sub

Sub AppStart (Args() As String)
    ' The Object field of a Type after Initialize: a new Object, as a local
    ' Dim gives (object_dim_default)? FerroBASIC gives Null meanwhile
    ' (docs/open-questions.md).
    Dim p As P
    p.Initialize
    Log(p.o = Null)
    Try
        Log(GetType(p.o))
    Catch
        Log("no type")
    End Try
End Sub
