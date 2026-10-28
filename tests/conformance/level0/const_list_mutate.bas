Sub Process_Globals
    ' Whether a List constant's contents may change (#33). A List made from
    ' an array has a fixed size, so Set, which it allows, asks the question.
    Private Const Names As List = Array("a", "b")
End Sub

Sub AppStart (Args() As String)
    Names.Set(0, "z")
    Log(Names.Get(0))
End Sub
