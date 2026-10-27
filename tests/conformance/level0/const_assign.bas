Sub Process_Globals
    ' Assigning a Const: B4A refuses it ("Cannot set a Const variable",
    ' forum); whether B4J does the same is to be recorded (#33).
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    Size = 11
    Log(Size)
End Sub
