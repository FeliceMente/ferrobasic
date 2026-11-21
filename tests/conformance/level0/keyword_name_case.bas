Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Case be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Case + 1) (Missing
    ' parameter.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Case that is declared but never read builds is open:
    ' keyword_name_case_dim_only. See docs/open-questions.md.
    Dim Case As Int = 4
    Log(Case + 1)
End Sub
