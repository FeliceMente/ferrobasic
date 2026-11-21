Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does b = Null build, compare, or raise a conversion error?
    ' Recording run 24: it builds, and when it runs it raises
    ' a RuntimeException (BA.ObjectToBoolean parses "null").
    ' FerroBASIC prints the same.
    ' See docs/spec/level0.md, comparisons.
    ' Expected output, one line per item below:
    '   nfe=false
    '   npe=false
    '   cce=false
    '   runtime=true
    Dim b As Boolean = False
    Try
        Log("value=" & (b = Null))
    Catch
        Log("nfe=" & LastException.Message.Contains("NumberFormatException"))
        Log("npe=" & LastException.Message.Contains("NullPointerException"))
        Log("cce=" & LastException.Message.Contains("ClassCastException"))
        Log("runtime=" & LastException.Message.Contains("RuntimeException"))
    End Try
End Sub
