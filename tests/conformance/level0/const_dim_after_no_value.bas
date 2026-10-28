Sub Process_Globals
    ' A Dim with no value after a Const of its name: built, and is the
    ' value still 10, or 0? FerroBASIC rejects it until a recording shows.
    Private Const Size As Int = 10
    Private Size As Int
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
