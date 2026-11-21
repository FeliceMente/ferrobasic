Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can $$ write a literal "$ inside a smart string? No:
    ' Recording run 24 shows that B4J refuses it (Syntax error.): the first "$ ends
    ' the string and leaves invalid trailing code. Concatenation writes "$
    ' (smart_literal_close_concat). FerroBASIC refuses it too.
    ' See docs/spec/level0.md, smart strings.
    Log($"L"$$R"$)
End Sub
