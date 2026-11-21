Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does a local variable named Return build when it is declared but never read?
    ' Recording run 24 shows that B4J refuses keyword_name_return at the read
    ' Log(Return + 1), not at its Dim. declared if the Dim alone builds; otherwise
    ' a build refusal, with no stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Dim Return As Int = 4
    Log("declared")
End Sub
