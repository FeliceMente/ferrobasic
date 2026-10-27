Sub Process_Globals
    ' The booklet's example of Const (2.3.3), word for word (#33). Other
    ' types and forms have tests of their own, so that one refusal cannot
    ' hide another answer.
    Private Const Size As Int = 10
    Public Const ItemNumber As Int = 100
End Sub

Sub AppStart (Args() As String)
    Log(Size)
    Log(ItemNumber + Size)
    Log(Twice(Size))
End Sub

Sub Twice (n As Int) As Int
    Return n * 2
End Sub
