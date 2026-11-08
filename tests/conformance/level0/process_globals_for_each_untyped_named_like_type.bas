Sub Process_Globals
    ' Recording run 21 (B4J 10.70): an untyped For Each over a Type's
    ' name declares a module variable Object, as over an ordinary name.
    ' AppStart reads its final value, 6.
    Type Point (X As Int)
    For Each Point In Array As Int(5, 6)
    Next
End Sub

Sub AppStart (Args() As String)
    Log(Point)
End Sub
