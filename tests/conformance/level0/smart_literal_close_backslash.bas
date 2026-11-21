Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can a backslash write a literal "$ inside a smart string?
    ' L"$R if a backslash escapes the closing delimiter; otherwise a build refusal:
    ' the first "$ ends the string and leaves invalid trailing code.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log($"L\"$R"$)
End Sub
