Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' How And and Or rank against = (docs/open-questions.md). Each line
    ' builds under any order; the comment gives what it prints if
    ' comparisons bind tighter / if And or Or binds tighter / if they share
    ' one level, read left to right.
    Log(False = False And False) ' false / true / false
    Log(False = False Or True)   ' true / false / true
    Log(False And False = False) ' false / true / true
    Log(True Or False = False)   ' true / false / false
End Sub
