Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Loop While is VB syntax, not supported by the reference toolchain.
    Dim n As Int = 0
    Do
        n = n + 1
    Loop While n < 3
End Sub
