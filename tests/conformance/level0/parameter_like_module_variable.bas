Sub Process_Globals
    Dim Count As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' A parameter with a module variable's name: accepted, hiding it (this
    ' prints 3, then 5), or refused as a local Dim of another type is?
    Show(3)
    Log(Count)
End Sub

Sub Show (Count As Int)
    Log(Count)
End Sub
