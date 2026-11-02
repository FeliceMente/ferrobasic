Sub Process_Globals
    Type Point (X As Int)
End Sub

Sub AppStart (Args() As String)
    ' GetType of a record (docs/open-questions.md).
    Dim p As Point
    p.Initialize
    Log(GetType(p))
End Sub
