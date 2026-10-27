Sub Process_Globals
    ' A Const with no As: which type it gets is open (#33). A Dim with no
    ' As is provisionally a String; GetType shows the constant's type, as a
    ' Java class name.
    Private Const Size = 10
End Sub

Sub AppStart (Args() As String)
    Log(GetType(Size))
    Log(Size + 1)
End Sub
