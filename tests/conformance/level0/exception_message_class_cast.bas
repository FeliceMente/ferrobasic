Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Recording run 25: does the class-cast message retain JDK loader context?
    ' The failing conversion is recorded in list_from_2d_array_object.
    ' JDK 19's checkcast text gives kind=true, cast=true, loader=true.
    ' FerroBASIC meanwhile gives kind=true, cast=true, loader=false.
    ' A bare CCE gives kind=true, cast=false, loader=false; a retained wrapper
    ' gives kind=false. Successful conversion prints "no exception".
    ' See docs/open-questions.md, JDK-generated exception messages.
    Dim Grid(2, 3) As Int
    Dim O As Object = Grid
    Dim Rows As List
    Try
        Rows.Initialize2(O)
        Log("no exception")
    Catch
        Log("kind=" & LastException.Message.StartsWith("java.lang.ClassCastException"))
        Log("cast=" & LastException.Message.Contains("cannot be cast"))
        Log("loader=" & LastException.Message.Contains("loader"))
    End Try
End Sub
