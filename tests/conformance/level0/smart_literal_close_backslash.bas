Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can a backslash write a literal "$ inside a smart string? No:
    ' Recording run 24 shows that B4J refuses it (Syntax error.): the backslash is
    ' an ordinary character and the "$ after it ends the string. Concatenation
    ' writes "$ (smart_literal_close_concat). FerroBASIC refuses it too.
    ' See docs/spec/level0.md, smart strings.
    Log($"L\"$R"$)
End Sub
