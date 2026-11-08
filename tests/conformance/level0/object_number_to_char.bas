Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 21 (B4J 10.70): BA.ObjectToChar takes the first
    ' character of the boxed number's text, so 65 gives "6".
    Dim o As Object = 65
    Try
        Dim c As Char = o
        Log(c)
    Catch
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
