Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A number held in an Object, assigned to a Char: if B4J converts with
    ' BA.ObjectToChar, which takes the first character of o.toString(), 65
    ' gives "6"; FerroBASIC raises ClassCastException meanwhile
    ' (docs/open-questions.md).
    Dim o As Object = 65
    Try
        Dim c As Char = o
        Log(c)
    Catch
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
