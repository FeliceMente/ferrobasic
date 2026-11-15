Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J groups this as (1 = 1) Or (2 = 3) and prints true
    ' (recording run 22).
    Log(1 = 1 Or 2 = 3)
End Sub
