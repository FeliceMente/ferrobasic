Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A number held in an Object, assigned to a Boolean: if B4J converts
    ' with BA.ObjectToBoolean, which parses String.valueOf(o), "1" raises
    ' RuntimeException ("Cannot parse: 1 as boolean"); FerroBASIC raises
    ' ClassCastException meanwhile (docs/open-questions.md). Only the kind
    ' is printed.
    Dim o As Object = 1
    Try
        Dim b As Boolean = o
        Log(b)
    Catch
        Log(LastException.Message.Contains("Cannot parse"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
