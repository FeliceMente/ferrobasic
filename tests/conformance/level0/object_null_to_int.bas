Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Null Object assigned to an Int: B4J's BA.ObjectToNumber parses
    ' String.valueOf(o), "null", a NumberFormatException rather than a
    ' NullPointerException, as null_number_compare records for a
    ' comparison (docs/research/level0-semantics.md, "Object to a number").
    Dim o As Object = Null
    Try
        Dim i As Int = o
        Log(i)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("NullPointerException"))
    End Try
End Sub
