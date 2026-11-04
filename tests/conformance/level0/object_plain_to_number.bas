Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A new Object (object_dim_default) assigned to an Int: B4J's
    ' BA.ObjectToNumber parses its text, a NumberFormatException, where
    ' FerroBASIC raises ClassCastException as for other objects that are
    ' not numbers (docs/open-questions.md). The message holds a hash that
    ' changes on every run, so only its kind is printed.
    Dim o As Object
    Try
        Dim i As Int = o
        Log(i)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
