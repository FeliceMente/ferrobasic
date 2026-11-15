Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Does B4J use Java's word boundaries when lowercasing capital sigma?
    ' Log each sigma as U+03C2 (962, final) or U+03C3 (963, medial).
    ' The JDK prints 962, 962, 963, 963, 963. FerroBASIC's Unicode 14
    ' context gives 963, 963, 962, 962, 962 (docs/open-questions.md).
    Log(Asc("A1Σ".ToLowerCase.CharAt(2)))
    Log(Asc("A_Σ".ToLowerCase.CharAt(2)))
    Log(Asc("AΣ_B".ToLowerCase.CharAt(1)))
    Log(Asc("A:Σ".ToLowerCase.CharAt(2)))
    Log(Asc("ªΣ".ToLowerCase.CharAt(1)))
End Sub
