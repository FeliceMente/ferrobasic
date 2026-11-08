Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' An initialised List held in an Object, assigned to an Int: B4J's
    ' BA.ObjectToNumber parses String.valueOf(o), "[1, 2]", a
    ' NumberFormatException rather than a ClassCastException
    ' (docs/research/level0-semantics.md, "Object conversions"). Only the
    ' kind is printed: a message can hold a hash.
    Dim l As List
    l.Initialize
    l.Add(1)
    l.Add(2)
    Dim o As Object = l
    Try
        Dim i As Int = o
        Log(i)
    Catch
        Log(LastException.Message.Contains("NumberFormatException"))
        Log(LastException.Message.Contains("ClassCastException"))
    End Try
End Sub
