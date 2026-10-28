Sub Process_Globals
    ' A Dim of a String Const's name after it: built as for an Int
    ' (const_dim_after)? FerroBASIC rejects it until a recording shows.
    Private Const Name As String = "a"
    Private Name As String = "b"
End Sub

Sub AppStart (Args() As String)
    Log(Name)
End Sub
