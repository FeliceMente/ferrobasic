Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    ' L1L3: Do the keywords accept a typed null array and an allocated empty array?
    ' Passing both references unchanged prints null=false,true then empty=true,false.
    ' Treating all typed arrays as initialized prints null=true,false; unwrapping a null
    ' array can print null=error. A compiler refusal has no stdout.
    ' FerroBASIC refuses both names today. See docs/open-questions.md, Level 1 library.
    Dim Nothing As Object = Null
    Dim U() As Int = Nothing
    Dim E() As Int = Array As Int()
    Try
        Log("null=" & Initialized(U) & "," & NotInitialized(U))
    Catch
        Log("null=error")
    End Try
    Try
        Log("empty=" & Initialized(E) & "," & NotInitialized(E))
    Catch
        Log("empty=error")
    End Try
End Sub
