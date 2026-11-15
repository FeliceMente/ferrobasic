Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts In as a local variable and prints 4 (recording run 22).
    Dim In As Int = 3
    Log(In + 1)
End Sub
