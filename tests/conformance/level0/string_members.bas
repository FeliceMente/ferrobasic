Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim s As String = "Hello, World"
    ' Positions and lengths count UTF-16 code units.
    Log(s.CharAt(4) & " " & s.Length & " " & "€".Length & " " & "😀".Length)
    Log(s.IndexOf("o") & " " & s.IndexOf2("o", 5) & " " & s.IndexOf("z"))
    Log(s.LastIndexOf("o") & " " & s.LastIndexOf2("o", 7) & " " & s.LastIndexOf("z"))
    Log(s.Contains("World") & " " & s.Contains("world"))
    Log(s.StartsWith("Hell") & " " & s.EndsWith("world") & " " & s.EqualsIgnoreCase("hello, world"))
    ' CompareTo is the difference of the first differing code units, or of the lengths.
    Log("apple".CompareTo("banana") & " " & "a".CompareTo("B") & " " & "abc".CompareTo("ab") & " " & "same".CompareTo("same"))
    Log(s.SubString(7) & "|" & s.SubString2(0, 5) & "|" & s.SubString2(3, 3) & "|")
    ' Replace is literal, left to right, without overlaps.
    Log("a.b.c".Replace(".", "-") & " " & "aaa".Replace("aa", "b") & " " & "x".Replace("", "-"))
    ' Trim removes code units up to U+0020 at both ends, not other spaces.
    Dim padded As String = " " & TAB & " x y " & CRLF
    Dim nbsp As String = Chr(160) & "z "
    Log("[" & padded.Trim & "] [" & nbsp.Trim & "] " & nbsp.Trim.Length)
    Log("Straße".ToUpperCase & " " & "ÀÉÎ".ToLowerCase & " " & "MiXeD".ToLowerCase)
    Dim b() As Byte = "aé€".GetBytes("UTF8")
    Log(b.Length & " " & b(0) & " " & b(1) & " " & b(2) & " " & b(5))
    Try
        Log(s.SubString2(5, 3))
    Catch
        Log("bad range")
    End Try
    Try
        Log(s.CharAt(12))
    Catch
        Log("bad index")
    End Try
End Sub
