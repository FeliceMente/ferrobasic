Sub Process_Globals
    ' A For counting over a Const: refused, as an assignment is? FerroBASIC
    ' rejects it until a recording shows.
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    For Size = 1 To 2
        Log(Size)
    Next
    Log(Size)
End Sub
