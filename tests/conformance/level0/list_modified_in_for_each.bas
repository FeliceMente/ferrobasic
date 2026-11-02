Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Adding to a List while a For Each iterates it
    ' (docs/open-questions.md).
    Dim l As List
    l.Initialize
    l.Add(1)
    l.Add(2)
    For Each n As Int In l
        Log(n)
        If l.Size < 4 Then l.Add(n + 10)
    Next
    Log(l)
End Sub
