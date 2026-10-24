Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' IIf evaluates only the branch it returns.
    Log(IIf(1 < 2, "first", Noisy("second")))
    Log(IIf(1 > 2, Noisy("first"), "second"))
    ' Open: the type of IIf's result, and so how it converts.
    Dim n As Int = IIf(True, 1, 2.5)
    Log(n)
    Dim d As Double = IIf(False, 1, 2.5)
    Log(d)
    Log(IIf(True, 3, 4.5))
End Sub

Sub Noisy (Text As String) As String
    Log("evaluated " & Text)
    Return Text
End Sub
