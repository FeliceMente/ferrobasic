Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    For i = 1 To 3
        Dim L As List
        L.Initialize
        L.Add(i)
        If i = 2 Then
            Dim inner As String = "set in block"
        End If
    Next
    Log(L.Size)
    Log(inner)
End Sub
