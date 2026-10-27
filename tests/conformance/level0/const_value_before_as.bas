Sub Process_Globals
    ' The value before As, as real projects write their constants (#33).
    Public Const Size = 3 As Int
End Sub

Sub AppStart (Args() As String)
    Log(Size)
End Sub
