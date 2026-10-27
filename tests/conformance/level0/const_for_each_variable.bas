Sub Process_Globals
    ' A For Each As in a Sub over a Const of the same type: refused, or a
    ' local of its own? FerroBASIC rejects it until a recording shows.
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    For Each Size As Int In Array As Int(5)
        Log(Size)
    Next
    Log(Size)
End Sub
