Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The type of a variable declared without `As`: String, or inferred
    ' (docs/open-questions.md)?
    Dim x = 5
    Log(GetType(x))
    Dim y
    y = 2.5
    Log(GetType(y))
End Sub
