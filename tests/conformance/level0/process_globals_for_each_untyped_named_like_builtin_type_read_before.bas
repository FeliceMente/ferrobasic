Sub Process_Globals
    ' Does a read of a built-in type's name in Process_Globals build before an
    ' untyped For Each over that name?
    ' Recording run 24 shows that a read before such a loop over an ordinary
    ' name sees it as Null (process_globals_for_each_untyped_read_before), and
    ' process_globals_for_each_untyped_named_like_builtin_type asks whether
    ' the loop builds over a built-in type's name at all. before=[null], then
    ' 6, if the read sees the variable too; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log("before=[" & Long & "]")
    For Each Long In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Long)
End Sub
