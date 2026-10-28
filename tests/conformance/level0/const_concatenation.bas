Sub Process_Globals
    ' A Const whose value is a concatenation of literals (#33).
    Private Const Name As String = "a" & "b"
End Sub

Sub AppStart (Args() As String)
    Log(Name)
End Sub
