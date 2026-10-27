Sub Process_Globals
    ' A Const whose value is an expression: operators, other constants,
    ' a concatenation (#33).
    Private Const Base As Int = 10
    Private Const Total As Int = 2 * 5 + 1
    Private Const Doubled As Int = Base * 2
    Private Const Name As String = "a" & "b"
End Sub

Sub AppStart (Args() As String)
    Log(Base)
    Log(Total)
    Log(Doubled)
    Log(Name)
End Sub
