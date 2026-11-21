Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does an initializer reading a name nothing declares or assigns build? No:
    ' Recording run 24 shows that B4J refuses it (Undeclared variable 'missing' is
    ' used before it was assigned any value.). FerroBASIC refuses it with
    ' the same words. The exact diagnostic is in the recording metadata.
    ' See docs/research/level0-semantics.md.
    Dim s As String = Missing
    Log("value=[" & s & "]")
End Sub
