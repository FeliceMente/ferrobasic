Sub Process_Globals
    ' The forms of Const the booklet shows (2.3.3): Private or Public Const
    ' Name As Type = value, here with five types (#33). The other forms
    ' have tests of their own, so that one refusal cannot hide another.
    Private Const Size As Int = 10
    Public Const ItemNumber As Int = 100
    Public Const Big As Long = 9000000000
    Private Const Rate As Double = 0.25
    Private Const Greeting As String = "Hello"
    Private Const Enabled As Boolean = True
End Sub

Sub AppStart (Args() As String)
    Log(Size)
    Log(ItemNumber + Size)
    Log(Big)
    Log(Rate * Size)
    Log(Greeting & " " & Size)
    Log(Enabled)
    Log(Twice(Size))
End Sub

Sub Twice (n As Int) As Int
    Return n * 2
End Sub
