Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' B4J binds = tighter than And and Or, regardless of which
    ' operator appears first (recording run 22).
    Log(False = False And False) ' false
    Log(False = False Or True) ' true
    Log(False And False = False) ' false
    Log(True Or False = False) ' true
End Sub
