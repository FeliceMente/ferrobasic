Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Select be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it ('end select' is missing., at End
    ' Sub): Dim Select starts a Select block. FerroBASIC refuses the Dim.
    ' The exact diagnostic is in the recording metadata.
    ' See docs/spec/level0.md, keywords as names.
    Dim Select As Int = 4
    Log(Select + 1)
End Sub
