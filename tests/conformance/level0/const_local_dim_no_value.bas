Sub Process_Globals
    ' A Dim with no value in a Sub with a module Const's name: refused, as
    ' with a value (const_local_dim)? FerroBASIC rejects it until a
    ' recording shows.
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    Dim Size As Int
    Log(Size)
End Sub
