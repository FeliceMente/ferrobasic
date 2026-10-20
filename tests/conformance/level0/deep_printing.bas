Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' How deeply nested Lists can be converted to text before Java's
    ' recursive toString overflows (docs/open-questions.md). Each depth that
    ' converts logs the text's length; the first one that fails stops the
    ' program, so a recording shows the threshold.
    Dim Depths() As Int = Array As Int(10, 100, 500, 1000, 2000, 5000, 10000, 100000)
    For Each Depth As Int In Depths
        Dim Head As List
        Head.Initialize
        For i = 2 To Depth
            Dim l As List
            l.Initialize
            l.Add(Head)
            Head = l
        Next
        Dim Text As String = Head
        Log(Depth & ": " & Text.Length)
    Next
End Sub
