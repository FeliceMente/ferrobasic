Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Comparing Strings with < and > (docs/open-questions.md).
    Log("a" < "b")
    Log("b" > "a")
    Log("B" < "a")
    Log("10" < "9")
End Sub
