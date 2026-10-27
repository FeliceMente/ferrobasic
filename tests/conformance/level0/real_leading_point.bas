Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A real literal may start with its decimal point, as a real-world
    ' project writes Array (320000,.08, .05, .03). Whether B4J accepts it,
    ' and as which type, is to be recorded.
    Log(.08)
    Log(.5 + 1)
    Log(-.25)
    Dim Rate As Double = .05
    Log(Rate * 100)
    Dim Values() As Object = Array(320000,.08, .05, .03)
    Log(Values(1))
    Log(1 - .5)
End Sub
