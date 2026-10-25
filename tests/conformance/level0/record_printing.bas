Sub Process_Globals
    Type One (A As Int)
    Type Two (A As Int, B As String)
    Type Five (A As Int, B As String, C As Double, D As Boolean, E As List)
End Sub

Sub AppStart (Args() As String)
    ' How a record prints (docs/open-questions.md): its IsInitialized field
    ' first, and, if jCore's BA.TypeToString is what prints it, a line break
    ' after every third field.
    Dim R1 As One
    R1.Initialize
    R1.A = 1
    Log(R1)
    Dim R2 As Two
    R2.Initialize
    R2.A = 2
    R2.B = "two"
    Log(R2)
    Dim R5 As Five
    R5.Initialize
    R5.A = 5
    R5.B = "five"
    R5.C = 1.5
    R5.D = True
    Log(R5)
    Dim Bare As Five
    Log(Bare)
    Log("A record in a text: " & R1)
End Sub
