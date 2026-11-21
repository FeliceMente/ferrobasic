Sub Process_Globals
    ' Does a read of a Type's name in Process_Globals build before an untyped
    ' For Each over that name?
    ' Recording run 21 shows the loop declares a module variable Object
    ' (process_globals_for_each_untyped_named_like_type), and run 24 that a
    ' read before such a loop over an ordinary name sees it as Null
    ' (process_globals_for_each_untyped_read_before). before=[null], then 6,
    ' if the read sees the variable too; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Type Point (X As Int)
    Log("before=[" & Point & "]")
    For Each Point In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Point)
End Sub
