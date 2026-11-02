Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does `If a = b = c` compile, and how does it group
    ' (docs/open-questions.md)? Grouped left, (1 = 2) = False is True;
    ' grouped right, 1 = (2 = False) compares an Int with a Boolean: an
    ' error, or False.
    Dim a As Int = 1
    Dim b As Int = 2
    Dim c As Boolean = False
    If a = b = c Then
        Log(True)
    Else
        Log(False)
    End If
End Sub
