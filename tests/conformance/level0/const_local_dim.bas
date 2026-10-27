Sub Process_Globals
    ' A Dim in a Sub with a module Const's name and type: refused, or a
    ' local of its own (a module variable's name gives that variable)?
    ' FerroBASIC rejects it until a recording shows.
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    Dim Size As Int = 4
    Log(Size)
End Sub
