Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Public be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Public + 1) (Syntax
    ' error.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Public that is declared but never read builds is open:
    ' keyword_name_public_dim_only. See docs/open-questions.md.
    Dim Public As Int = 4
    Log(Public + 1)
End Sub
