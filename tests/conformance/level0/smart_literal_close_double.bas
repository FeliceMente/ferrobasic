Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can doubling the quote write a literal "$ inside a smart string?
    ' L"$R if doubling escapes the closing delimiter; otherwise a build refusal:
    ' the first "$ ends the string and leaves invalid trailing code.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log($"L""$R"$)
End Sub
