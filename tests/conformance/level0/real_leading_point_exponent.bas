Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A real literal that starts with its decimal point, with an exponent.
    ' Whether B4J accepts it is to be recorded; real_leading_point records
    ' the form without one.
    Log(.5e2)
    Log(.25E-1)
    Log(-.5e+1)
End Sub
