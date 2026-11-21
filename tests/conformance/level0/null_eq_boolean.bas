Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does Null = b build, compare, or raise a conversion error?
    ' Recording run 24: it builds, and when it runs it raises
    ' a NullPointerException.
    ' FerroBASIC prints the same.
    ' See docs/spec/level0.md, comparisons.
    ' Expected output, one line per item below:
    '   nfe=false
    '   npe=true
    '   cce=false
    '   runtime=false
    Dim b As Boolean = False
    Try
        Log("value=" & (Null = b))
    Catch
        Log("nfe=" & LastException.Message.Contains("NumberFormatException"))
        Log("npe=" & LastException.Message.Contains("NullPointerException"))
        Log("cce=" & LastException.Message.Contains("ClassCastException"))
        Log("runtime=" & LastException.Message.Contains("RuntimeException"))
    End Try
End Sub
