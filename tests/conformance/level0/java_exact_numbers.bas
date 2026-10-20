Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Inverse trigonometry is Java's fdlibm code, to the last bit.
    Log(ATan(-0.49018109740036864))
    Log(ASin(0.9381944945543601))
    Log(ACos(-0.3327795776506606))
    Log(ATan2(0.4617563814065817, -0.17983837701559668))
    Log(ATan2(1E20, -1))
    Log(ATan2(1E300, -1))
    ' Degrees: divided by Pi, then multiplied by 180.
    Log(ASinD(0.001))
    Log(ACosD(0.001))
    Log(ATanD(0.03))
    Log(ATan2D(0.001, -1))
    ' A Float exactly halfway between two shortest decimals: the even one.
    Dim f As Float = 27.899368
    Log(f)
    ' NumberFormat takes its digits from an older conversion.
    Log(NumberFormat(1234567890123456789, 0, 0))
    Log(NumberFormat(Power(2, 64), 0, 0))
    Log(NumberFormat(Power(2, 70), 0, 0))
    Log(NumberFormat2(Power(2, -20), 1, 30, 0, False))
    ' Select on an Object that holds a number compares numerically.
    Dim o As Object = 1
    Select o
        Case 1.0
            Log("one")
        Case Else
            Log("not one")
    End Select
    Dim s As Object = "1"
    Select s
        Case 1
            Log("number")
        Case "1"
            Log("string")
    End Select
    Try
        Select o
            Case "x"
                Log("x")
        End Select
    Catch
        Log("not a number")
    End Try
    Try
        Select o
            Case Null
                Log("null")
        End Select
    Catch
        Log("null case")
    End Try
    Dim n As Object = Null
    Try
        Select n
            Case 1
                Log("1")
        End Select
    Catch
        Log("null scrutinee")
    End Try
End Sub
