Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The type of a variable an assignment declares (undeclared_variable):
    ' a String, as old forum posts say, or the value's (docs/open-questions.md)?
    Undeclared = 5
    Log(GetType(Undeclared))
    Undeclared = Undeclared + 1
    Log(Undeclared)
End Sub
