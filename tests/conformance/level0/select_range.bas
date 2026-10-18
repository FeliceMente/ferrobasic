Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Case ranges are VB syntax, not supported by the reference toolchain.
    Dim v As Int = 2
    Select v
        Case 1 To 4
            Log("small")
    End Select
End Sub
