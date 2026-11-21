Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Return be a local variable name, like In and Each?
    ' 5 if the local declaration and arithmetic read build; otherwise a build refusal, with no
    ' stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Dim Return As Int = 4
    Log(Return + 1)
End Sub
