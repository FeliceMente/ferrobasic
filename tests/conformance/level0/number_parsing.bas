Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Bit.ParseInt accepts any Unicode decimal digit, and fullwidth letters.
    Log(Bit.ParseInt("٣٤", 10))
    Log(Bit.ParseInt("１２", 10))
    Log(Bit.ParseInt("ｆＦ", 16))
    Log(Bit.ParseInt("-१०", 10))
    Try
        Log(Bit.ParseInt("½", 10))
    Catch
        Log("not a digit")
    End Try
    ' IsNumber parses with Double.parseDouble, which refuses Null.
    Log(IsNumber("1e3") & " " & IsNumber("0x10"))
    Dim Missing As String = Null
    Try
        Log(IsNumber(Missing))
    Catch
        Log("null text")
    End Try
End Sub
