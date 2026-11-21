Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does a local variable named Else build when it is declared but never read?
    ' Recording run 24 shows that B4J refuses keyword_name_else at the read
    ' Log(Else + 1), not at its Dim. declared if the Dim alone builds; otherwise
    ' a build refusal, with no stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Dim Else As Int = 4
    Log("declared")
End Sub
