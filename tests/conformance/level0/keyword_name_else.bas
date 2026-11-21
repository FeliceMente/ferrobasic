Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Else be a local variable name, like In and Each?
    ' 5 if the local declaration and arithmetic read build; otherwise a build refusal, with no
    ' stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Dim Else As Int = 4
    Log(Else + 1)
End Sub
