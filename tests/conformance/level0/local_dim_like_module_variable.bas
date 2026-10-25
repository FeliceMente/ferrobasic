Sub Process_Globals
    Dim Count As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' A local Dim with a module variable's name and type: a new local (this
    ' prints 5) or the module variable itself (7)? FerroBASIC rejects it
    ' until a recording shows.
    SetCount
    Log(Count)
End Sub

Sub SetCount
    Dim Count As Int
    Count = 7
End Sub
