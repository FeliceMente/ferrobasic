Sub Process_Globals
    ' A variable declared again by a Const builds (const_after_dim): may it
    ' then be assigned, or is it a constant now? FerroBASIC rejects it until
    ' a recording shows.
    Private Size As Int = 3
    Private Const Size As Int = 10
End Sub

Sub AppStart (Args() As String)
    Size = 5
    Log(Size)
End Sub
