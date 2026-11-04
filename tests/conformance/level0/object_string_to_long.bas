Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A String with a fraction held in an Object, assigned to a Long and to
    ' an Int: does B4J convert to a Long with BA.ObjectToLongNumber
    ' (Long.parseLong, a NumberFormatException) or with ObjectToNumber
    ' (Double.parseDouble, 1)? docs/open-questions.md
    Dim o As Object = "1.5"
    Try
        Dim n As Long = o
        Log(n)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
    End Try
    Dim i As Int = o
    Log(i)
End Sub
