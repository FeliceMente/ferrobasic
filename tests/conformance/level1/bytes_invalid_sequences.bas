Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L2: How does the recording JDK decode malformed UTF-8 and unmappable US-ASCII?
    ' Replacement prints utf8=65533,40 and ascii=65533; dropping prints utf8=40 and ascii=;
    ' errors print utf8=error or ascii=error. Other policies print their UTF-16 units.
    ' FerroBASIC refuses this program today. See docs/open-questions.md, Level 1 library.
    Dim U() As Byte = Array As Byte(-61, 40)
    Dim A() As Byte = Array As Byte(-1)
    Try
        Log("utf8=" & Units(BytesToString(U, 0, U.Length, "UTF-8")))
    Catch
        Log("utf8=error")
    End Try
    Try
        Log("ascii=" & Units(BytesToString(A, 0, A.Length, "US-ASCII")))
    Catch
        Log("ascii=error")
    End Try
End Sub

Sub Units (S As String) As String
    Dim Result As String = ""
    For I = 0 To S.Length - 1
        If I > 0 Then Result = Result & ","
        Result = Result & Asc(S.CharAt(I))
    Next
    Return Result
End Sub
