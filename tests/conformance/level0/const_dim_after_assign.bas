Sub Process_Globals
    ' A Const declared again by a Dim builds (const_dim_after): may the
    ' variable then be assigned, or is it still a constant? FerroBASIC
    ' rejects it until a recording shows.
    Private Const Size As Int = 10
    Private Size As Int = 3
End Sub

Sub AppStart (Args() As String)
    Size = 5
    Log(Size)
End Sub
