Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Log(Bit.And(12, 10) & " " & Bit.Or(12, 10) & " " & Bit.Xor(12, 10) & " " & Bit.Not(12))
    Log(Bit.And(-1, 255) & " " & Bit.Or(-256, 15) & " " & Bit.Xor(-1, 0))
    ' Shift counts are taken modulo 32, as in Java.
    Log(Bit.ShiftLeft(1, 4) & " " & Bit.ShiftLeft(1, 31) & " " & Bit.ShiftLeft(1, 33))
    Log(Bit.ShiftRight(-16, 2) & " " & Bit.UnsignedShiftRight(-16, 28) & " " & Bit.ShiftRight(256, 36))
    ' Negative numbers print as their 32-bit two's complement.
    Log(Bit.ToHexString(255) & " " & Bit.ToHexString(-1) & " " & Bit.ToHexString(0))
    Log(Bit.ToBinaryString(10) & " " & Bit.ToBinaryString(-8))
    Log(Bit.ToOctalString(8) & " " & Bit.ToOctalString(-1))
End Sub
