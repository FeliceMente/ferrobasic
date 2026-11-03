Sub Process_Globals
    ' A loop variable of Process_Globals named like a Type
    ' (process_globals_loop_named_like_type): is the Type still usable, and
    ' does the name read as the variable (docs/open-questions.md)?
    Type Point (X As Int)
    For Point = 1 To 2
    Next
End Sub

Sub AppStart (Args() As String)
    Dim p As Point
    p.Initialize
    p.X = 4
    Log(p.X)
    Log(Point)
End Sub
