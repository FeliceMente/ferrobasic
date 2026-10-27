Sub Process_Globals
    ' A String constant (#33).
    Private Const Greeting As String = "Hello"
End Sub

Sub AppStart (Args() As String)
    Log(Greeting & "!")
End Sub
