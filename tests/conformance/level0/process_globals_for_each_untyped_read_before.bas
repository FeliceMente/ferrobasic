Sub Process_Globals
    ' Does an untyped For Each in Process_Globals build after a read of its otherwise
    ' undeclared name?
    ' Build refusal, or seen=5, seen=6, after=6 if the loop declares a shared module Object;
    ' before=[null] (or before=[]) distinguishes the value before the loop. A different after=
    ' value identifies the value AppStart sees.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log("before=[" & u & "]")
    For Each u In Array As Int(5, 6)
        Log("seen=" & u)
    Next
End Sub

Sub AppStart (Args() As String)
    Log("after=" & u)
End Sub
