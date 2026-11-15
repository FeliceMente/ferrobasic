Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Do comparisons bind tighter than And (docs/open-questions.md)? If so,
    ' this is (1 < 2) And (3 < 4) and prints true. If And binds tighter it
    ' is the chain 1 < (2 And 3) < 4, and on one level, left to right,
    ' ((1 < 2) And 3) < 4 compares a Boolean: either way it does not build.
    Log(1 < 2 And 3 < 4)
End Sub
