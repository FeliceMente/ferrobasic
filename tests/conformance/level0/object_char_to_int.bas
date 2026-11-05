Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Char held in an Object, assigned to an Int: B4J's BA.ObjectToNumber
    ' parses String.valueOf(o), the character itself, so "7" gives 7 and
    ' "A" a NumberFormatException rather than a ClassCastException
    ' (docs/research/level0-semantics.md, "Object to a number").
    Dim c As Char = "7"
    Dim o As Object = c
    Try
        Dim i As Int = o
        Log(i)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
    Dim a As Char = "A"
    o = a
    Try
        Dim j As Int = o
        Log(j)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
