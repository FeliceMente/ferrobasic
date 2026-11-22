Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L7: Are literal invalid date/time patterns diagnosed at compile time or by the runtime
    ' setters?
    ' Runtime validation prints date=true then time=true; a compiler refusal has no stdout;
    ' accepting a setter prints date=accepted or time=accepted; other errors print false.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    Try
        DateTime.DateFormat = "invalid"
        Log("date=accepted")
    Catch
        Log("date=" & LastException.Message.Contains("IllegalArgumentException"))
    End Try
    Try
        DateTime.TimeFormat = "invalid"
        Log("time=accepted")
    Catch
        Log("time=" & LastException.Message.Contains("IllegalArgumentException"))
    End Try
End Sub
