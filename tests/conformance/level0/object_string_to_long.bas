Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 21 (B4J 10.70): ObjectToLongNumber uses
    ' Long.parseLong, so "1.5" raises NumberFormatException for a Long;
    ' ObjectToNumber still parses it as a Double for an Int, giving 1.
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
