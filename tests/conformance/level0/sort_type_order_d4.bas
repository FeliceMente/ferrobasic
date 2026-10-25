Sub Process_Globals
    Type A (N As Object)
    Type B (N As Object)
End Sub
Sub AppStart (Args() As String)
    Dim X As A
    X.Initialize
    X.N = 1
    Dim Y As A
    Y.Initialize
    Y.N = "x"
    Dim Z As B
    Z.Initialize
    Z.N = 1
    Dim L As List
    L.Initialize
    L.Add(X)
    L.Add(Y)
    L.Add(Z)
    ' Two problems: X and Y cannot be compared (Int and String), and Z is
    ' another Type. Java's sort meets the first, FerroBASIC the second.
    Try
        L.SortType("N", True)
    Catch
        Log(LastException.Message.StartsWith("java.lang.RuntimeException"))
        Log(LastException.Message.Contains("ClassCast"))
        Log(LastException.Message.Contains("IllegalArgument"))
    End Try
End Sub
