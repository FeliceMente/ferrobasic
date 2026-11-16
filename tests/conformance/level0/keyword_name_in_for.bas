Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts In as a counted For variable and prints 1 and 2
    ' (recording run 22).
    For In = 1 To 2
        Log(In)
    Next
End Sub
