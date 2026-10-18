Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim s As String = ""
    For k = 1 To 10
        If k Mod 2 = 0 Then Continue
        s = s & k
    Next
    Log(s)
End Sub
