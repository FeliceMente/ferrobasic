Sub Process_Globals
    ' Assigning a field of a Const record: refused, as an array item is,
    ' or allowed, as List.Set is? FerroBASIC rejects it until a recording
    ' shows.
    Type Point (X As Int)
    Private Const P As Point = NewPoint
End Sub

Sub AppStart (Args() As String)
    P.X = 4
    Log(P.X)
End Sub

Sub NewPoint As Point
    Dim q As Point
    q.Initialize
    Return q
End Sub
