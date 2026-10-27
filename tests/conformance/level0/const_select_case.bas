Sub Process_Globals
    ' Constants as Select Case labels, as real projects use them (#33).
    Private Const Push As Int = 1
    Private Const Pop As Int = 2
End Sub

Sub AppStart (Args() As String)
    Dim Code As Int = 2
    Select Code
        Case Push
            Log("push")
        Case Pop
            Log("pop")
    End Select
End Sub
