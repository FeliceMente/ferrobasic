Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J accept Each as a parameter (docs/open-questions.md)? It is
    ' recorded only as a local variable (keyword_name_each). If it builds,
    ' this prints 10; otherwise B4J refuses it. FerroBASIC accepts it
    ' meanwhile.
    Log(Twice(5))
End Sub

Sub Twice (Each As Int) As Int
    Return Each * 2
End Sub
