Sub Process_Globals
    ' A module variable read in Process_Globals before its Dim there: B4J
    ' refuses a read before the declaration in a Sub
    ' (for_each_untyped_read_before). Here it builds, and the read gives
    ' Null (recording run 19).
    Log(n)
    Dim n As Object
End Sub

Sub AppStart (Args() As String)
    Log(GetType(n))
End Sub
