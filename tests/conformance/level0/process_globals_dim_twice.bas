Sub Process_Globals
    ' Two Dims of one name and type in Process_Globals: built, and which
    ' value does it keep (docs/open-questions.md)? Found by ferro-compare:
    ' FerroBASIC accepts it, and no recording settles it.
    Private Size As Int = 3
    Private Size As Int = 4
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
