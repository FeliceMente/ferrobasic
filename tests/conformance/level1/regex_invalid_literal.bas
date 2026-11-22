Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L6: Does B4J validate a literal regex before execution or delegate it to the runtime?
    ' Runtime validation prints pattern=true; a compiler refusal has no stdout; no validation
    ' prints accepted; other exceptions print pattern=false.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    Try
        Log(Regex.IsMatch("[", "x"))
        Log("accepted")
    Catch
        Log("pattern=" & LastException.Message.Contains("PatternSyntaxException"))
    End Try
End Sub
