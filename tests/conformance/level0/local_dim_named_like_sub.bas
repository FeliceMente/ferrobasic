Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A local Dim with a Sub's name: B4J refuses it, as it refuses a module
    ' variable's.
    Dim Bump As Int = 4
    Log(Bump)
End Sub

Sub Bump
End Sub
