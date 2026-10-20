Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Log("before")
    ' ExitApplication2 ends the process at once: Try does not catch it.
    Try
        Stop(3)
    Catch
        Log("caught")
    End Try
    Log("after")
End Sub

Sub Stop (Code As Int)
    Log("stopping")
    ExitApplication2(Code)
    Log("not reached")
End Sub
