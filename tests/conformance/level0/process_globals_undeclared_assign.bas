Sub Process_Globals
    ' Does an assignment in Process_Globals declare a module variable when
    ' nothing else declares the name?
    ' Recording run 24 shows that B4J refuses
    ' process_globals_for_each_untyped_assign_before because the untyped loop's
    ' Object does not match the earlier declaration, which suggests that u = 9
    ' declares a String. after=9 and type=java.lang.String if it declares a
    ' module String that AppStart sees; otherwise a build refusal, with no stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    u = 9
End Sub

Sub AppStart (Args() As String)
    Log("after=" & u)
    Log("type=" & GetType(u))
End Sub
