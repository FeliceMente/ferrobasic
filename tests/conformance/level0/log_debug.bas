Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does LogDebug print in release mode (docs/open-questions.md)?
    Log("before")
    LogDebug("debug")
    Log("after")
End Sub
