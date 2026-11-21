Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does a local variable named Public build when it is declared but never read?
    ' Recording run 24 shows that B4J refuses keyword_name_public at the read
    ' Log(Public + 1), not at its Dim. declared if the Dim alone builds; otherwise
    ' a build refusal, with no stdout.
    ' See docs/open-questions.md.
    ' FerroBASIC meanwhile refuses this program.
    Dim Public As Int = 4
    Log("declared")
End Sub
