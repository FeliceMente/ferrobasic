Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L8: Can date/time smart formatters bind Object operands holding numeric text, as the
    ' runtime helper allows?
    ' Runtime coercion prints d=true then t=true then dt=true; rejecting Object at compile
    ' time gives no stdout; a runtime error prints error; differing formatter semantics prints
    ' false.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    DateTime.DateFormat = "'D'"
    DateTime.TimeFormat = "'T'"
    Dim O As Object = "0"
    Try
        Log("d=" & ($"$date{O}"$ = "D"))
        Log("t=" & ($"$time{O}"$ = "T"))
        Log("dt=" & ($"$datetime{O}"$ = "D T"))
    Catch
        Log("error")
    End Try
End Sub
