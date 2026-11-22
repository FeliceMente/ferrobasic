Sub Process_Globals
    Type Item (N As Int)
End Sub

Sub AppStart (Args() As String)
    ' L1L5: Do the keywords observe a generated Type record initialization field through
    ' Object?
    ' Reflection prints unset=false,true then ready=true,false; null-only testing prints
    ' unset=true,false; errors print the corresponding label=error.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    Dim R As Item
    Try
        Dim Before As Object = R
        Log("unset=" & Initialized(Before) & "," & NotInitialized(Before))
    Catch
        Log("unset=error")
    End Try
    R.Initialize
    Try
        Dim After As Object = R
        Log("ready=" & Initialized(After) & "," & NotInitialized(After))
    Catch
        Log("ready=error")
    End Try
End Sub
