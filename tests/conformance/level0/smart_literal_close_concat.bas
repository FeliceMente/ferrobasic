Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can a literal "$ be built by concatenation inside a smart string? Yes:
    ' Recording run 24 records L"$R. This is the way to write "$, since a smart
    ' string has no escape for it (smart_literal_close_double).
    ' FerroBASIC prints the same.
    ' See docs/spec/level0.md, smart strings.
    ' Expected output: L"$R
    Log($"L${QUOTE & "$"}R"$)
End Sub
