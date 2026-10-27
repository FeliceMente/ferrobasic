Sub Process_Globals
    ' A Const of type List (#33).
    Private Const Names As List = Array("a", "b")
End Sub

Sub AppStart (Args() As String)
    Log(Names.Size)
End Sub
