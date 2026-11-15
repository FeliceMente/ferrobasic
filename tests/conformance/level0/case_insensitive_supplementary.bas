Sub Process_Globals
    Type Word (Text As String, Id As Int)
End Sub

' Case-insensitive comparison of characters outside the BMP (issue #115).
' Deseret U+10400 and U+10428 are a case pair, written as surrogate pairs.
' U+10401 is the next capital letter. Prints only booleans and indexes.
Sub AppStart (Args() As String)
    Dim up As String = Chr(0xD801) & Chr(0xDC00)
    Dim low As String = Chr(0xD801) & Chr(0xDC28)
    Dim up2 As String = Chr(0xD801) & Chr(0xDC01)
    Dim lone As String = Chr(0xDC28)
    ' EqualsIgnoreCase, supplementary pairs.
    Log(up.EqualsIgnoreCase(low))
    Log(low.EqualsIgnoreCase(up))
    Log(("a" & up & "b").EqualsIgnoreCase("A" & low & "B"))
    Log(up.EqualsIgnoreCase(up2))
    Log(low.EqualsIgnoreCase(up2))
    ' BMP pairs.
    Log("ÀÉÎ".EqualsIgnoreCase("àéî"))
    Log("Σσς".EqualsIgnoreCase("σΣΣ"))
    Log("ǅ".EqualsIgnoreCase("ǆ"))
    Log("ǅ".EqualsIgnoreCase("Ǆ"))
    Log("Straße".EqualsIgnoreCase("STRASSE"))
    ' Unpaired surrogates compare as themselves.
    Log(lone.EqualsIgnoreCase(lone))
    Log(lone.EqualsIgnoreCase(Chr(0xDC00)))
    Log((Chr(0xD801) & "x").EqualsIgnoreCase(Chr(0xD801) & "X"))
    Log((Chr(0xD801) & "x").EqualsIgnoreCase(Chr(0xD802) & "X"))
    Log((lone & up).EqualsIgnoreCase(lone & low))

    ' Sorts: each line gives the original index of each item, in order.
    ' Indexes 0 and 2 are equal ignoring case, as are 3 and 5; 6 is an
    ' unpaired low surrogate.
    Dim items As List
    items.Initialize
    items.AddAll(Array As String(low, up2, up, "B", "a", "b", lone))
    Dim sorted As List
    sorted.Initialize
    sorted.AddAll(items)
    sorted.SortCaseInsensitive(True)
    Log(Indexes(items, sorted))
    sorted.Clear
    sorted.AddAll(items)
    sorted.SortCaseInsensitive(False)
    Log(Indexes(items, sorted))

    Dim words As List
    words.Initialize
    For i = 0 To items.Size - 1
        Dim w As Word
        w.Initialize
        w.Text = items.Get(i)
        w.Id = i
        words.Add(w)
    Next
    Dim sortedWords As List
    sortedWords.Initialize
    sortedWords.AddAll(words)
    sortedWords.SortTypeCaseInsensitive("Text", True)
    Log(Ids(sortedWords))
    sortedWords.Clear
    sortedWords.AddAll(words)
    sortedWords.SortTypeCaseInsensitive("Text", False)
    Log(Ids(sortedWords))
End Sub

Sub Indexes (Original As List, Sorted As List) As String
    Dim sb As StringBuilder
    sb.Initialize
    For i = 0 To Sorted.Size - 1
        If i > 0 Then sb.Append(" ")
        sb.Append(Original.IndexOf(Sorted.Get(i)))
    Next
    Return sb.ToString
End Sub

Sub Ids (Sorted As List) As String
    Dim sb As StringBuilder
    sb.Initialize
    For i = 0 To Sorted.Size - 1
        Dim w As Word = Sorted.Get(i)
        If i > 0 Then sb.Append(" ")
        sb.Append(w.Id)
    Next
    Return sb.ToString
End Sub
