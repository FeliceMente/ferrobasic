Sub Process_Globals
    Type Box (Items As List)
End Sub

Sub AppStart (Args() As String)
    ' A record whose List holds the record itself: does printing it end,
    ' print "(this Collection)", or overflow (docs/open-questions.md)?
    Dim b As Box
    b.Initialize
    b.Items.Initialize
    b.Items.Add(b)
    Log("built")
    Log(b)
End Sub
