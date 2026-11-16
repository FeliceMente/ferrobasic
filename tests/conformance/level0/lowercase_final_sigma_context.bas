Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 23 confirms that B4J lowercases capital sigma by
    ' Java's word boundaries: each sigma is U+03C2 (962, final) or
    ' U+03C3 (963, medial), and B4J prints 962, 962, 963, 963, 963.
    ' Unicode's own context would give 963, 963, 962, 962, 962.
    Log(Asc("A1Σ".ToLowerCase.CharAt(2)))
    Log(Asc("A_Σ".ToLowerCase.CharAt(2)))
    Log(Asc("AΣ_B".ToLowerCase.CharAt(1)))
    Log(Asc("A:Σ".ToLowerCase.CharAt(2)))
    Log(Asc("ªΣ".ToLowerCase.CharAt(1)))
End Sub
