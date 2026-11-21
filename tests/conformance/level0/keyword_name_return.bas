Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Return be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Return + 1) (Missing
    ' parameter.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Return that is declared but never read builds is open:
    ' keyword_name_return_dim_only. See docs/open-questions.md.
    Dim Return As Int = 4
    Log(Return + 1)
End Sub
