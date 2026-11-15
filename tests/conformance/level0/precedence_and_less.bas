Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J groups this as (1 < 2) And (3 < 4) and prints true
    ' (recording run 22).
    Log(1 < 2 And 3 < 4)
End Sub
