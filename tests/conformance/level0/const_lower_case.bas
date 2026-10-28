Sub Process_Globals
    ' Const in lower case, as a real project writes it (#33).
    Private const Size As Int = 3
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
