Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' Only a one-dimensional array converts to a List: the original
    ' toolchain casts a 2-dimensional one, which its Java compiler rejects
    ' ("int[][] cannot be converted to List").
    Dim Grid(2, 3) As Int
    Dim Rows As List
    Rows.Initialize2(Grid)
End Sub
