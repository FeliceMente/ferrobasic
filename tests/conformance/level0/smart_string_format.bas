Sub Process_Globals
End Sub

Sub AppStart (Args() As String)
    Dim h = 2, m = 15, s = 7 As Int
    Log($"Remaining time $2{h}:$2{m}:$2{s}"$)
    Log($"10 / 7 = $0.3{10 / 7}"$)
    Log($"$1.2{"The value is not a number!"}"$)
    Log($"5 * 3 = ${5 * 3}"$)
    Log($"$1.2{12345.678}"$)
    Log($"$1.2{0.125}"$)
    Log($"$XML{"<a & 'b'>"}"$)
    Log($"No need to escape "quotes"!"$)
    Log(NumberFormat(12345.6789, 0, 2))
End Sub
