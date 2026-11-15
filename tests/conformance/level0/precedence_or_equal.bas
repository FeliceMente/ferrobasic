Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Do comparisons bind tighter than Or (docs/open-questions.md)? If so,
    ' this is (1 = 1) Or (2 = 3) and prints true. If Or binds tighter it
    ' is the chain 1 = (1 Or 2) = 3, which does not build. On one level,
    ' left to right, it is ((1 = 1) Or 2) = 3, a number given to Or.
    Log(1 = 1 Or 2 = 3)
End Sub
