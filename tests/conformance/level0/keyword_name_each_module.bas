Sub Process_Globals
    Private Each As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts Each as a module variable and prints 6 (recording run 22).
    Log(Each + 1)
End Sub
