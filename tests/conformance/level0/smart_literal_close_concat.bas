Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can & inside a placeholder write a literal "$ inside a smart string?
    ' L"$R if the concatenated delimiter stays literal; otherwise a build refusal if the
    ' generated delimiter ends the string.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   L"$R
    Log($"L${QUOTE & "$"}R"$)
End Sub
