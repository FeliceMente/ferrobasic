Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim sb As StringBuilder
    Log(sb.IsInitialized)
    sb.Initialize
    Log(sb.IsInitialized & " " & sb.Length & " [" & sb.ToString & "]")
    ' Append returns the StringBuilder, so calls chain.
    sb.Append("Hello").Append(", ").Append("World")
    Log(sb.ToString & " " & sb.Length)
    sb.Insert(5, "!!")
    Log(sb.ToString)
    ' Remove takes a start and an end offset, end exclusive.
    sb.Remove(5, 7)
    Log(sb.ToString)
    ' Numbers and Booleans are converted as for &.
    Dim n As Int = 42
    Dim d As Double = 1.5
    sb.Append(" ").Append(n).Append(" ").Append(d).Append(" ").Append(True)
    Log(sb.ToString)
    ' An end past the length stops at the length.
    sb.Remove(5, 1000)
    Log(sb.ToString & " " & sb.Length)
    Try
        sb.Insert(99, "x")
    Catch
        Log("bad offset")
    End Try
    Try
        sb.Remove(4, 2)
    Catch
        Log("bad range")
    End Try
    Log(sb.ToString)
End Sub
