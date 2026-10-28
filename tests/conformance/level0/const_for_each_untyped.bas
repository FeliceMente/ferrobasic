Sub Process_Globals
    ' A For Each without As over a Const: refused, as one with As is
    ' (const_for_each_variable)? FerroBASIC rejects it until a recording
    ' shows.
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    For Each Size In Array As Int(5)
        Log(Size)
    Next
    Log(Size)
End Sub
