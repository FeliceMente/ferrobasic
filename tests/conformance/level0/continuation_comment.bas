Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' May a comment follow a line continuation? FerroBASIC rejects it until a
    ' recording shows.
    Dim Total As Int = 1 + _ ' first part
        2
    Log(Total)
End Sub
