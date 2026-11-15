Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts Each as the variable of an untyped For Each and prints
    ' 3 and 4 (recording run 22).
    For Each Each In Array As Int(3, 4)
        Log(Each)
    Next
End Sub
