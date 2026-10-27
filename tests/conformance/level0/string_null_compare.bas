Sub Process_Globals
    Type R (S As String)
End Sub

Sub AppStart (Args() As String)
    ' A Null assigned to a String is the text "null" (BA.ObjectToString);
    ' how does a String compare with Null (docs/open-questions.md)?
    Dim S As String = Null
    Log(S = Null)
    Log(S = "null")
    Dim Unset As R
    Log(Unset.S = Null)
    Dim O As Object = Null
    Log(O = Null)
End Sub
