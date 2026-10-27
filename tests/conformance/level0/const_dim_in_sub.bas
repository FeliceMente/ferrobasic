Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Dim Const inside a Sub (#33).
    Dim Const Size As Int = 5
    Log(Size * 2)
End Sub
