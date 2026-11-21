Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can doubling the braces write a literal ${ inside a smart string? No:
    ' Recording run 24 shows that B4J refuses it (Missing parameter.): doubled braces
    ' are not an escape. A smart string has no escape for ${; concatenation
    ' writes it (smart_literal_open_concat). FerroBASIC refuses it too.
    ' See docs/spec/level0.md, smart strings.
    Log($"L${{5}}R"$)
End Sub
