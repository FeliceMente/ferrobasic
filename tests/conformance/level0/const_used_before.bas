Sub Process_Globals
    ' A variable initialised from a Const declared after it: 20 if B4J
    ' inlines constants, 0 if it reads the variable before it is set, or a
    ' compile error (#33).
    Private Twice As Int = Size * 2
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    Log(Twice)
    Log(Size)
End Sub
