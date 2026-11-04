Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The value of `Dim o As Object` with no value: recording run 16 printed
    ' java.lang.Object@<hash> after one (for_each_untyped_object_declared_after),
    ' a new Object, as FerroBASIC gives (docs/open-questions.md).
    ' The hash changes on every run, so only its type and Null are printed.
    Dim o As Object
    Log(o = Null)
    Try
        Log(GetType(o))
    Catch
        Log("no type")
    End Try
End Sub
