' The main module has no Sub Process_Globals: the original toolchain
' refuses to build it ("Sub process_globals is missing").
Sub AppStart (Args() As String)
    Log("never runs")
End Sub
