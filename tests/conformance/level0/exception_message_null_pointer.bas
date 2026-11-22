Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 25: does LastException.Message carry a helpful JDK NPE?
    ' A JEP 358 message gives kind=true, detail=true, because=true.
    ' A bare NPE gives kind=true, detail=false, because=false, as FerroBASIC
    ' does meanwhile. Other detail text gives detail=true, because=false.
    ' No error prints the returned type and then "no exception".
    ' See docs/open-questions.md, JDK-generated exception messages.
    Try
        Log(GetType(Null))
        Log("no exception")
    Catch
        Log("kind=" & LastException.Message.StartsWith("java.lang.NullPointerException"))
        Log("detail=" & LastException.Message.Contains(": "))
        Log("because=" & LastException.Message.Contains("because"))
    End Try
End Sub
