Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can $$ write a literal "$ inside a smart string?
    ' L"$R if $$ escapes the closing delimiter; L"$$R if both dollars stay
    ' literal; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log($"L"$$R"$)
End Sub
