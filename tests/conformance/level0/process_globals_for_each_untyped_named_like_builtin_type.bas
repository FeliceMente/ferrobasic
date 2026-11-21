Sub Process_Globals
    ' Does an untyped For Each in Process_Globals over a built-in type's name
    ' declare a module variable Object, as it does over a Type's name?
    ' Recording run 21 shows it over a Type's name
    ' (process_globals_for_each_untyped_named_like_type); no recording shows
    ' a built-in type's name, such as Long, as a loop variable. 6 if it
    ' does; otherwise a build refusal.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile builds it, as over a Type's name, and prints:
    '   6
    For Each Long In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Long)
End Sub
