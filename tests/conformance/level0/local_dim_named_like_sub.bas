Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' A local Dim with a Sub's name: accepted, or refused as a module
    ' variable's is? FerroBASIC rejects it until a recording shows.
    Dim Bump As Int = 4
    Log(Bump)
End Sub

Sub Bump
End Sub
