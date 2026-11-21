Sub Process_Globals
    ' Does an untyped For Each in Process_Globals build after an assignment to its
    ' otherwise undeclared name?
    ' Build refusal, or seen=5, seen=6, after=6 if the loop declares a shared module Object;
    ' after=9 means AppStart sees the earlier assignment instead of the loop's last item.
    ' A different after= value identifies the value AppStart sees.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    u = 9
    For Each u In Array As Int(5, 6)
        Log("seen=" & u)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("after=" & u)
End Sub
