Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L1: Does Length=-1 with a nonzero offset mean the whole array or its remainder?
    ' Source predicts bounds=true; remainder semantics prints text=BC. Other errors print
    ' bounds=false.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    Dim B() As Byte = Array As Byte(65, 66, 67)
    Try
        Log("text=" & BytesToString(B, 1, -1, "UTF-8"))
    Catch
        Log("bounds=" & LastException.Message.Contains("IndexOutOfBoundsException"))
    End Try
End Sub
