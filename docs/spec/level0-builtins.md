# Level 0 built-ins

The keywords and library members a Level 0 program can use, and that the
interpreter must provide.

- Signatures follow the official
  [B4J Core reference](https://www.b4x.com/b4j/help/core.html).
- Behaviour follows the Apache-2.0 B4J runtime source (`jCore`, commit
  `e26e645a`; see [../research/level0-semantics.md](../research/level0-semantics.md#sources)).
- Names are case-insensitive.

When the M3 library-descriptor reader lands, this list must agree with the
`jCore` `.xml` descriptor. The descriptor wins.

A member not listed here gets a precise diagnostic. For example:
`'DateTime.Now' is not supported at Level 0 (needs the DateTime object, Level 1)`.

## Keywords and constants

| Member | Behaviour |
| --- | --- |
| `Log(Message As String)` | stdout, plus `\n`; `Null` prints `null` |
| `LogError(Message As String)` | stderr, plus `\n` |
| `LogDebug(Message As String)` | like `Log` *(open: B4J only logs it in debug mode; release mode is the reference, so it prints nothing)* |
| `True`, `False`, `Null` | literals |
| `CRLF`, `TAB`, `QUOTE` | `Chr(10)`, `Chr(9)`, `Chr(34)` |
| `cPI`, `cE` | Java `Math.PI`, `Math.E` |
| `Not(Value As Boolean) As Boolean` | logical not |
| `IIf(Condition, TrueValue, FalseValue)` | evaluates only the chosen branch. The result type is `Object` unless `.As(T)` is used *(open)*. |
| `Abs`, `Sqrt`, `Power`, `Floor`, `Ceil`, `Sin`, `Cos`, `Tan`, `ASin`, `ACos`, `ATan`, `ATan2`, `SinD`, `CosD`, `TanD`, `ASinD`, `ACosD`, `ATanD`, `ATan2D`, `Logarithm(Number, Base)`, `Max`, `Min` | `Double` in and out, Java `Math` semantics; `ASin`, `ACos`, `ATan` and `ATan2` match Java bit for bit (fdlibm). The `D` variants use degrees: `ASinD(x)` is `ASin(x) / π * 180`, in that order. |
| `Round(Number As Double) As Long` | Java `Math.round`: halves round up (`Round(-2.5)` is `-2`) |
| `Round2(Number As Double, DecimalPlaces As Int) As Double` | `Math.round(x * 10^d) / 10^d` |
| `NumberFormat(Number, MinimumIntegers, MaximumFractions)` | see [level0.md](level0.md#strings) |
| `NumberFormat2(Number, MinimumIntegers, MaximumFractions, MinimumFractions, GroupingUsed)` | same, with a minimum number of fraction digits and optional grouping |
| `IsNumber(Text As String) As Boolean` | `True` if Java `Double.parseDouble` accepts the text |
| `Asc(Char As Char) As Int`, `Chr(UnicodeValue As Int) As Char` | UTF-16 code unit ↔ Int |
| `Rnd(Min As Int, Max As Int) As Int`, `RndSeed(Seed As Long)` | `Min + nextInt(Max - Min)` on the `java.util.Random` algorithm (a 48-bit LCG), so a seeded sequence matches B4J exactly. Unseeded output is not compared. |
| `ExitApplication`, `ExitApplication2(ExitCode As Int)` | end the process with code 0 / `ExitCode` |
| `LastException As Exception` | the error caught by the current `Catch`. Members: `Message As String`, `IsInitialized As Boolean`. |
| `Array [As T](values…)` | a new one-dimensional array; `Object` if `T` is omitted |
| `CreateMap(k1: v1, k2: v2, …) As Map` | an initialised `Map` with the pairs in order |
| `GetType(object As Object) As String` | *(open)*: B4J returns the Java class name (`java.lang.Integer`). Level 0 returns the same names for primitives, boxes, `String`, `List` and `Map`; other values are a deviation. |
| `x.As(T)` | inline cast, same rules as assigning to a variable of type `T` |
| `Bit.And`, `Bit.Or`, `Bit.Xor`, `Bit.Not`, `Bit.ShiftLeft`, `Bit.ShiftRight`, `Bit.UnsignedShiftRight`, `Bit.ToHexString`, `Bit.ToBinaryString`, `Bit.ToOctalString`, `Bit.ParseInt(Value, Radix)` | Java `int` semantics |

Not in Level 0: `Sleep`, `CallSub*`, `CallSubDelayed*`, `Sender`, `Me`,
`StartMessageLoop`/`StopMessageLoop`, `File`, `DateTime`, `Regex`, `Timer`,
`SubExists`, `BytesToString`, `CharsToString`, `GetEnvironmentVariable`,
`GetSystemProperty`/`SetSystemProperty`, `IsDevTool`, `Density`, `DipToCurrent`.

## String

Strings are immutable; every member returns a new value.

| Member | Behaviour (Java `String`) |
| --- | --- |
| `CharAt(Index As Int) As Char` | `charAt` |
| `CompareTo(Other As String) As Int` | `compareTo` (UTF-16 order; upper case before lower case) |
| `Contains(SearchFor As String) As Boolean` | `contains` |
| `StartsWith(Prefix)`, `EndsWith(Suffix)` | |
| `EqualsIgnoreCase(Other As String) As Boolean` | |
| `IndexOf(SearchFor)`, `IndexOf2(SearchFor, Index)`, `LastIndexOf(SearchFor)`, `LastIndexOf2(SearchFor, Index)` | -1 if not found |
| `Length As Int` | number of UTF-16 code units |
| `Replace(Target, Replacement) As String` | replaces all occurrences, literally (not a regex) |
| `SubString(BeginIndex)`, `SubString2(BeginIndex, EndIndex)` | end exclusive; out of range → `StringIndexOutOfBoundsException` |
| `ToLowerCase`, `ToUpperCase` | Java's no-argument versions, which use the JVM default locale. FerroBASIC uses the Unicode default (root-locale) mapping, which equals Java for every locale except Turkish, Azeri and Lithuanian (`"I"` → `ı` in Turkish); *(open)* which locale the reference recordings use. |
| `Trim As String` | Java `trim`: removes code units ≤ U+0020 at both ends |

`GetBytes(Charset)` returns `Byte()` and is Level 0 only for `"UTF8"`/`"UTF-8"`.

## StringBuilder

`Initialize`, `Append(Text) As StringBuilder`, `Insert(Offset, Text) As StringBuilder`,
`Remove(StartOffset, EndOffset) As StringBuilder`, `Length As Int`,
`ToString As String`, `IsInitialized As Boolean`.

## List

Wraps a Java `ArrayList`. Every member except `Initialize`, `Initialize2` and
`IsInitialized` on an uninitialised list raises
`Object should first be initialized (List).`

| Member | Behaviour |
| --- | --- |
| `Initialize` | new empty list |
| `Initialize2(Array As List)` | shares the given list. An array argument gives a fixed-size list: `Add`/`RemoveAt` then raise `UnsupportedOperationException`. |
| `IsInitialized As Boolean` | |
| `Add(Item)`, `AddAll(List)`, `AddAllAt(Index, List)`, `InsertAt(Index, Item)` | |
| `Get(Index) As Object`, `Set(Index, Item)`, `RemoveAt(Index)` | out of range → `IndexOutOfBoundsException` |
| `Clear`, `Size As Int` | |
| `IndexOf(Item) As Int` | `equals` comparison, so boxed types matter |
| `SubList(BeginIndex, EndIndex) As List` | a read-only view backed by the list (`Collections.unmodifiableList(subList(...))`, class `java.util.Collections$UnmodifiableRandomAccessList`): `Set` on the parent shows through; after a structural change of the parent (`Add`, `RemoveAt`, `InsertAt`, `Clear`, `AddAll`, `Sort`…) any access through the view raises `java.util.ConcurrentModificationException`; `Clear`, `Sort` and other changes through the view raise `java.lang.UnsupportedOperationException` |
| `Sort(Ascending As Boolean)` | Java `Comparable` order, stable. Items must all have the same boxed type: all `String`s, or all numbers of one type. Mixing types, e.g. `Int` and `Double`, raises `ClassCastException`. |
| `SortCaseInsensitive(Ascending As Boolean)` | compares the items' `toString()` with `compareToIgnoreCase`, stable. Numbers are therefore sorted as text: `2, 10` → `10, 2`, and a boxed Double `1` compares as `"1.0"`. |

`SortType(FieldName, Ascending)` and `SortTypeCaseInsensitive(FieldName, Ascending)`
apply the same two rules to the named field of `Type` records (the field name
is case-sensitive).

## Map

Wraps a Java `LinkedHashMap`: iteration is in insertion order, and replacing a
value keeps the key's position. Keys compare with `equals`: Strings are
case-sensitive, and boxes of different numeric types are different keys.

| Member | Behaviour |
| --- | --- |
| `Initialize`, `IsInitialized As Boolean` | |
| `Put(Key, Value) As Object` | returns the previous value or `Null` |
| `Get(Key) As Object` | `Null` if missing |
| `GetDefault(Key, Default) As Object` | `Default` if missing or `Null` |
| `Remove(Key) As Object`, `ContainsKey(Key) As Boolean`, `Clear`, `Size As Int` | |
| `Keys`, `Values` | for `For Each`; a snapshot iterator is *(open)* |
| `GetKeyAt(Index)`, `GetValueAt(Index)` | deprecated but supported; positional, in insertion order |

## Printing collections

`Log(list)` and `"" & map` print the wrapper's `toString`, for example
`(ArrayList) [1, 2]` and `(MyMap) {a=1, b=2}`; an uninitialised one prints
`(List) Not initialized` / `(Map) Not initialized`. The element formatting inside
(Java's `toString` of each element, so `1.0` keeps its `.0`) and the exact
type names are *(open)*.
