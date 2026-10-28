Sub Process_Globals
    ' Dim Const with one name in Process_Globals (#33).
    Dim Const Size As Int = 1
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
