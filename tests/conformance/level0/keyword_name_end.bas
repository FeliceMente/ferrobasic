Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can End be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at the Dim (end as is not a valid
    ' identifier.). FerroBASIC refuses it there too.
    ' The exact diagnostic is in the recording metadata.
    ' See docs/spec/level0.md, keywords as names.
    Dim End As Int = 4
    Log(End + 1)
End Sub
