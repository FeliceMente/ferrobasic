Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 21 (B4J 10.70): BA.ObjectToBoolean parses the
    ' number's text and raises RuntimeException ("Cannot parse: 1 as boolean"),
    ' not ClassCastException. Only the exception kind is printed.
    Dim o As Object = 1
    Try
        Dim b As Boolean = o
        Log(b)
    Catch
        Log(LastException.Message.Contains("Cannot parse"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
