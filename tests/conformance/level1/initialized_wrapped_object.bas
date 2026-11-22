Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L4: Does passing an unset wrapper directly and as Object preserve the same
    ' initialization result?
    ' Preserved state prints direct=false,true then boxed=false,true; loss of wrapper state
    ' can print true,false or error for either case.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    Dim L As List
    Try
        Log("direct=" & Initialized(L) & "," & NotInitialized(L))
    Catch
        Log("direct=error")
    End Try
    Try
        Dim O As Object = L
        Log("boxed=" & Initialized(O) & "," & NotInitialized(O))
    Catch
        Log("boxed=error")
    End Try
End Sub
