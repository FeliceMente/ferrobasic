Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' May a comment follow a line continuation with no space before it?
    ' FerroBASIC rejects it until a recording shows.
    Dim Total As Int = 1 + _' first part
        2
    Log(Total)
End Sub
