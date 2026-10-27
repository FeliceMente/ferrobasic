Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' The type of a real literal that starts with its decimal point, as
    ' real_leading_point writes them: Double, as 0.08 is? To be recorded.
    Log(GetType(.08))
    Log(GetType(0.08))
End Sub
