Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can a backslash write a literal "$ inside a smart string?
    ' L"$R if a backslash escapes the closing delimiter; L\"$R if the backslash
    ' stays literal; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log($"L\"$R"$)
End Sub
