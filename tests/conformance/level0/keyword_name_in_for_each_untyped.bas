Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J accepts In immediately before the In delimiter of an untyped
    ' For Each and prints 3 and 4 (recording run 22).
    For Each In In Array As Int(3, 4)
        Log(In)
    Next
End Sub
