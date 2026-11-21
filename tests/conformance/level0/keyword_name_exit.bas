Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Exit be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Exit + 1) (Missing
    ' parameter.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Exit that is declared but never read builds is open:
    ' keyword_name_exit_dim_only. See docs/open-questions.md.
    Dim Exit As Int = 4
    Log(Exit + 1)
End Sub
