Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does `If a = b = c` compile, and how does it group
    ' (docs/open-questions.md)?
    Dim a As Int = 1
    Dim b As Int = 1
    If a = b = True Then
        Log("grouped left")
    Else
        Log("not")
    End If
End Sub
