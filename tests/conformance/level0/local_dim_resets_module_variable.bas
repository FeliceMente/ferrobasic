Sub Process_Globals
    Dim Count As Int = 5
End Sub

Sub AppStart (Args() As String)
    ' A local Dim with a module variable's name and type is that module
    ' variable (recorded): does the Dim reset it (this prints 0) or leave it
    ' (5)?
    Reset
    Log(Count)
End Sub

Sub Reset
    Dim Count As Int
End Sub
