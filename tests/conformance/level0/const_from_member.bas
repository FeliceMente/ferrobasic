Sub Process_Globals
    ' A Const whose value comes from a member of a literal (#33).
    Private Const Length As Int = "abc".Length
End Sub

Sub AppStart (Args() As String)
    Log(Length)
End Sub
