Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Else be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Else + 1) (Missing
    ' parameter.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Else that is declared but never read builds is open:
    ' keyword_name_else_dim_only. See docs/open-questions.md.
    Dim Else As Int = 4
    Log(Else + 1)
End Sub
