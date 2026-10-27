Sub Process_Globals
    ' Dim Const with several names, as the booklet's Dim syntax shows (#33).
    Dim Const First = 1, Second = 2 As Int
End Sub

Sub AppStart (Args() As String)
    Log(First + Second)
End Sub
