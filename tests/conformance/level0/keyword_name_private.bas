Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Can Private be a local variable name, like In and Each? No:
    ' Recording run 24 shows that B4J refuses it at Log(Private + 1) (Syntax
    ' error.), after the Dim line. FerroBASIC refuses the Dim already.
    ' Whether a Private that is declared but never read builds is open:
    ' keyword_name_private_dim_only. See docs/open-questions.md.
    Dim Private As Int = 4
    Log(Private + 1)
End Sub
