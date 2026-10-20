Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim x As Double = -2.5
    Log(Abs(x) & " " & Sqrt(2) & " " & Sqrt(-1))
    Log(Ceil(2.1) & " " & Ceil(-2.1) & " " & Floor(2.9) & " " & Floor(-2.1) & " " & Ceil(-0.5))
    ' Round is Java's Math.round: halves round up, towards positive infinity.
    Log(Round(2.5) & " " & Round(-2.5) & " " & Round(-2.6) & " " & Round(0.49999999999999994))
    Dim nan As Double = Sqrt(-1)
    Log(Round(nan))
    ' Round2 rounds x * 10^d, then divides by 10^d.
    Log(Round2(3.14159, 2) & " " & Round2(2.675, 2) & " " & Round2(1234.5678, 0) & " " & Round2(-1.005, 2))
    ' Max and Min: NaN wins, and -0 is less than 0.
    Dim zero As Double = 0
    Dim negZero As Double = -zero
    Log(Max(3, 7.5) & " " & Min(3, 7.5) & " " & Max(nan, 1) & " " & Min(negZero, zero) & " " & Max(negZero, zero))
    Log(cPI & " " & cE)
    Log(Power(2, 10) & " " & Power(9, 0.5) & " " & Power(2, -1) & " " & Logarithm(1, 10))
End Sub
