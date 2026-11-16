Sub Process_Globals
    Private In As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts In as a module variable and prints 6 (recording run 22).
    Log(In + 1)
End Sub
