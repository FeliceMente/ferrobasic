Sub Process_Globals
    ' Does an untyped For Each in Process_Globals build after a read of its otherwise
    ' undeclared name? Yes: recording run 24 shows the loop declares a module
    ' Object that the earlier read already sees, as Null, and that AppStart
    ' sees after the loop. After an assignment instead, B4J refuses it
    ' (process_globals_for_each_untyped_assign_before). FerroBASIC prints the same.
    ' See docs/spec/level0.md, For Each.
    ' Expected output, one line per item below:
    '   before=[null]
    '   seen=5
    '   seen=6
    '   after=6
    Log("before=[" & u & "]")
    For Each u In Array As Int(5, 6)
        Log("seen=" & u)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("after=" & u)
End Sub
