Sub Process_Globals
    ' Does an untyped For Each in Process_Globals build after an assignment to its
    ' otherwise undeclared name? No.
    ' Recording run 24 shows that B4J refuses it at the For Each (Current declaration
    ' does not match previous one.): the assignment declares u a String, and
    ' the untyped loop then declares it an Object, as in the recorded local
    ' case for_each_untyped_assigned_before.
    ' FerroBASIC refuses it too, at the assignment: it does not declare a
    ' module variable by assignment in Process_Globals (an open question:
    ' process_globals_undeclared_assign).
    ' See docs/open-questions.md.
    ' The exact diagnostic is in the recording metadata.
    u = 9
    For Each u In Array As Int(5, 6)
        Log("seen=" & u)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("after=" & u)
End Sub
