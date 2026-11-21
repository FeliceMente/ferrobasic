Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can a literal ${ be built by concatenation inside a smart string? Yes:
    ' Recording run 24 records L${5}R. This is the way to write ${, since a smart
    ' string has no escape for it (smart_literal_open_double).
    ' FerroBASIC prints the same.
    ' See docs/spec/level0.md, smart strings.
    ' Expected output: L${5}R
    Log($"L${"$" & "{"}5}R"$)
End Sub
