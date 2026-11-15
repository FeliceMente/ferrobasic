Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts In as a Sub name and prints 8 (recording run 22).
    Log(In + 1)
End Sub

Sub In As Int
    Return 7
End Sub
