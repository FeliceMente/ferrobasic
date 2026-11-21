Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can a backslash write a literal ${ inside a smart string?
    ' L${5}R if a backslash escapes interpolation; L\5R if the backslash is literal and 5
    ' interpolates; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile prints, one line per item below:
    '   L\5R
    Log($"L\${5}R"$)
End Sub
