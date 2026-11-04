Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Float, a Short and a Byte `=` Null: a NumberFormatException each, as
    ' with an Int, a Double and a Long (null_number_compare, double_eq_null,
    ' long_eq_null)? FerroBASIC refuses them (docs/open-questions.md).
    Dim f As Float = 0
    Dim s As Short = 0
    Dim b As Byte = 0
    Try
        Log(f = Null)
    Catch
        Log("Float: " & LastException.Message)
    End Try
    Try
        Log(s = Null)
    Catch
        Log("Short: " & LastException.Message)
    End Try
    Try
        Log(b = Null)
    Catch
        Log("Byte: " & LastException.Message)
    End Try
End Sub
