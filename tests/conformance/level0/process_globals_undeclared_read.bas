Sub Process_Globals
    ' Does a Process_Globals read of a name nothing declares or assigns build?
    ' No: Recording run 24 shows that B4J refuses it (Undeclared variable 'missing'
    ' is used before it was assigned any value.), as in a Sub. FerroBASIC
    ' refuses it with the same words. The exact diagnostic is in the metadata.
    ' See docs/research/level0-semantics.md.
    Log("value=[" & Missing & "]")
End Sub

Sub AppStart (Args() As String)
    Log("app")
End Sub
