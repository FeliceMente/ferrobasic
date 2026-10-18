Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim d As Double
    d = 3
    Log(d)
    d = 2.5
    Log(d)
    d = 10000000
    Log(d)
    d = 1234567.5
    Log(d)
    d = 0.001
    Log(d)
    d = 0.0001
    Log(d)
    d = 0.1 + 0.2
    Log(d)
    d = 123456789012.0
    Log(d)
    Dim f As Float = 1.1
    Log(f)
    Dim i As Int = -42
    Log(i)
    Dim b As Boolean = True
    Log(b)
End Sub
