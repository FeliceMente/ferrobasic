Sub Process_Globals
    ' Two Consts of one name and type: refused, or built as a Const and a
    ' Dim are (const_dim_after)? FerroBASIC rejects it until a recording
    ' shows.
    Private Const Size As Int = 10
    Private Const Size As Int = 3
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
