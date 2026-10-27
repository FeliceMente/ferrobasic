Sub Process_Globals
    ' A Const array with bounds and no value: refused, as a Const with no
    ' value is, or an array of default items? FerroBASIC rejects it until a
    ' recording shows.
    Private Const Slots(3) As Int
End Sub

Sub AppStart (Args() As String)
    Log(Slots.Length)
End Sub
