Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Continue be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Continue + 1) (Missing
    ' parameter.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Continue that is declared but never read builds is open:
    ' keyword_name_continue_dim_only. See docs/open-questions.md.
    Dim Continue As Int = 4
    Log(Continue + 1)
End Sub
