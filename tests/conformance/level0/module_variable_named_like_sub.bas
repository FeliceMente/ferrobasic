Sub Process_Globals
    Dim Bump As Int = 4
End Sub

Sub AppStart (Args() As String)
    ' A module variable declared with a Sub's name: B4J refuses the Dim.
    Log(Bump)
End Sub

Sub Bump As Int
    Return 9
End Sub
