Sub Process_Globals
    ' A Dim of a Const's name after it, inside an If: built as when it is
    ' not (const_dim_after)? FerroBASIC rejects it until a recording
    ' shows.
    Private Const Size As Int = 10
    If True Then
        Private Size As Int = 3
    End If
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
