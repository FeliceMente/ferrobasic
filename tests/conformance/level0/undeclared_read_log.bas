Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does a direct Log read of a name nothing declares or assigns build?
    ' Build refusal if a never-declared, never-assigned name cannot be read; value=[] for an
    ' implicit empty String, value=[null] for Null, or the default value between brackets.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Log("value=[" & Missing & "]")
End Sub
