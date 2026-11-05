Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A Boolean held in an Object, assigned to a Double and to a Long:
    ' B4J's BA.ObjectToNumber and ObjectToLongNumber parse
    ' String.valueOf(o), "true", a NumberFormatException rather than a
    ' ClassCastException (docs/research/level0-semantics.md, "Object to a
    ' number").
    Dim o As Object = True
    Try
        Dim d As Double = o
        Log(d)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
    Try
        Dim n As Long = o
        Log(n)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
