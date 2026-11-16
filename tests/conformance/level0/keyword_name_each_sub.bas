Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts Each as a Sub name and prints 8 (recording run 22).
    Log(Each + 1)
End Sub

Sub Each As Int
    Return 7
End Sub
