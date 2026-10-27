Sub Process_Globals
    ' The forms of Const that public sources show (#33): Private and
    ' Public Const (booklet 2.3.3), lower case, the value before As (as
    ' real projects write it), and Dim Const with several names (the
    ' booklet's Dim syntax).
    Private Const Size As Int = 10
    Public Const ItemNumber As Int = 100
    Private const Lower = 3 As Int
    Public Const Big = 9000000000 As Long
    Private Const Rate As Double = 0.25
    Private Const Greeting As String = "Hello"
    Private Const Enabled = True As Boolean
    Dim Const First = 1, Second = 2 As Int
End Sub

Sub AppStart (Args() As String)
    ' Dim Const inside a Sub.
    Dim Const Local As Int = 5
    Log(Size)
    Log(ItemNumber + Lower)
    Log(Big)
    Log(Rate * Size)
    Log(Greeting & " " & Size)
    Log(Enabled)
    Log(First + Second)
    Log(Local * Size)
    Log(Twice(Size))
End Sub

Sub Twice (n As Int) As Int
    Return n * 2
End Sub
