Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Calling a member on an Object: a compile error (docs/open-questions.md)?
    Dim o As Object = "abc"
    Log(o.Length)
End Sub
