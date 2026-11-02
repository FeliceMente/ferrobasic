Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Which boxed type does a numeric literal get as an Object, and are
    ' Int 1 and Double 1.0 different Map keys (docs/open-questions.md)?
    Dim o As Object = 1
    Log(GetType(o))
    o = 1.0
    Log(GetType(o))
    Dim m As Map
    m.Initialize
    m.Put(1, "int")
    m.Put(1.0, "double")
    Log(m.Size)
    Log(m.Get(1))
End Sub
