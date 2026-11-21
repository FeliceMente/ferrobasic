Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can $$ write a literal ${ inside a smart string? No:
    ' Recording run 24 records L$5R. The first $ is an ordinary character and
    ' ${5} still interpolates; a smart string has no escape for ${.
    ' FerroBASIC prints the same.
    ' See docs/spec/level0.md, smart strings.
    ' Expected output: L$5R
    Log($"L$${5}R"$)
End Sub
