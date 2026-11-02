Sub Process_Globals
    ' The second Dim of the name has no value: does it reset the variable
    ' to 0 (docs/open-questions.md)? Found by ferro-compare.
    Private Size As Int = 3
    Private Size As Int
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
