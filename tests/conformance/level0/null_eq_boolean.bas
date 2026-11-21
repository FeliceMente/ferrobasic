Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does Null = b build, compare, or raise a conversion error?
    ' Build refusal, or value=true/false for a comparison result. A caught conversion error
    ' prints nfe=true for NumberFormatException, npe=true for NullPointerException, cce=true
    ' for ClassCastException, or runtime=true for RuntimeException; other kind tests are
    ' false.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
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
