Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Log(Asc("A") & " " & Asc("€") & " " & Chr(66) & Chr(8364) & " " & Asc(Chr(65535)))
    ' CRLF is a single line feed; TAB and QUOTE are one character each.
    Dim s As String = "a" & CRLF & "b" & TAB & QUOTE
    Log(s.Length & " " & Asc(s.CharAt(1)) & " " & Asc(s.CharAt(3)) & " " & Asc(s.CharAt(4)))
    Log(Not(True) & " " & Not(1 > 2))
    ' Integer arithmetic wraps around, as in Java.
    Dim big As Int = 2147483647
    Log(big + 1)
    Dim huge As Long = 9223372036854775807
    Log(huge + 1)
    ' Int times Int is an Int, even when the result is stored in a Long.
    Dim product As Long = big * 2
    Log(product)
    ' Long to Int keeps the low 32 bits.
    Dim wide As Long = 4294967297
    Dim narrow As Int = wide
    Log(narrow)
    ' Byte and Short arithmetic is done as Int.
    Dim b As Byte = 127
    Dim sh As Short = 32767
    Log((b + 1) & " " & (sh + 1))
    Log((-7 Mod 3) & " " & (7 Mod -3))
    Try
        Dim zero As Int = 0
        Log(7 Mod zero)
    Catch
        Log(LastException.Message.Contains("ArithmeticException"))
    End Try
End Sub
