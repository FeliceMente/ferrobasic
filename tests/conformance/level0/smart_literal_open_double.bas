Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can doubling the braces write a literal ${ inside a smart string?
    ' L${5}R if doubled braces escape interpolation; L5R if a braced expression
    ' evaluates to 5; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log($"L${{5}}R"$)
End Sub
