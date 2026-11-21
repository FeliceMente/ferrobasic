Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can & inside a placeholder write a literal ${ inside a smart string?
    ' L${5}R if the concatenated delimiter stays literal; L5R if the result is interpolated
    ' again; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   L${5}R
    Log($"L${"$" & "{"}5}R"$)
End Sub
