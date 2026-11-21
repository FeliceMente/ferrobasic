Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can doubling the quote write a literal "$ inside a smart string?
    ' L"$R if doubling escapes the closing delimiter; L""$R if both quotes stay
    ' literal; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log($"L""$R"$)
End Sub
