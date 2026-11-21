Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Exit be a local variable name, like In and Each?
    ' 5 if the local declaration and arithmetic read build; otherwise a build refusal, with no
    ' stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Dim Exit As Int = 4
    Log(Exit + 1)
End Sub
