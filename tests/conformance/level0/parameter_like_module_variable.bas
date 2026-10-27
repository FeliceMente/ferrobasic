Sub Process_Globals
    Dim Count As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' A parameter with a module variable's name: B4J refuses it.
    Show(3)
    Log(Count)
End Sub

Sub Show (Count As Int)
    Log(Count)
End Sub
