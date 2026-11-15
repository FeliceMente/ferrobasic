Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts Each as a parameter and prints 10 (recording run 22).
    Log(Twice(5))
End Sub

Sub Twice (Each As Int) As Int
    Return Each * 2
End Sub
