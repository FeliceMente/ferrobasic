# Level 1 library: findings and recording questions

Task C of [#146](https://github.com/FeliceMente/ferrobasic/issues/146),
November 2026. This covers the single-module library work in
[the M2 plan](../roadmap.md#m2-task-breakdown), independently of classes,
events and the multi-module recorder. Nothing here is implemented. The eight
probes below await **recording run 25**, B4J 10.70 on Windows, OpenJDK 19.
There are no hand-written expected or provisional outputs. A recording
supersedes these findings.

## Sources and boundaries

As in [Level 0 research](level0-semantics.md#sources), **source** means
runtime implementation, **docs** official documentation, **forum** an
Anywhere Software answer, **generated** compiler-produced Java, and
*(open)* an unresolved question. Each rule below links its evidence.

- **source**: Apache-2.0 [jCore at commit
  `e26e645a611dd24c2034fbbeaf11ba26b70dee8b`][jcore]. `Common` declares
  `@Version(10.70f)`. Source establishes runtime calls, not whether the
  compiler accepts a particular source expression or wraps its argument.
- **docs**: [B4J Core reference][core], [B4J Regex reference][regex-docs],
  and *B4X Language* v2.5 in [Booklets.zip][booklets], especially §4.2
  (conversions), §4.10.4 (smart strings), §7.14 (invalid byte conversions).
  The older references omit optional-parameter annotations present in
  the pinned source. Signatures below retain those annotations.
- **docs (JDK 19)**: the Java SE 19 specifications linked at each
  delegation. They describe the recording JDK's API, not a newer Java
  version. An unspecified Java behavior stays open here.
- **forum**: only answers by Erel (Anywhere Software), linked per rule.
  Cross-product answers are identified and do not establish a B4J-specific
  compiler detail.
- **generated**: no B4J-generated Java settling the open bindings below
  was found in the official booklet examples. No rule here is promoted
  to this label from another product's generated Java.

The declarations are Java declarations from the pinned source (public,
static and final modifiers omitted in tables). Java `boolean`, `int`,
`long`, `double`, `String`, arrays map to Boolean, Int, Long, Double,
String, `T()` in library signatures. `getX`/`setX` expose property `X`.
The tables also show the B4J spelling where that is less obvious.

## Regex methods

Declarations: **source**, [Regex.java, lines 41–132][regex-source].
`Regex` is predefined; it is not a user-created instance (**docs**,
[B4J Regex reference][regex-docs]).

| jCore declaration | B4J surface |
| --- | --- |
| `int CASE_INSENSITIVE = Pattern.CASE_INSENSITIVE` | read-only Int constant (2) |
| `int MULTILINE = Pattern.MULTILINE` | read-only Int constant (8) |
| `boolean IsMatch(String Pattern, String Text)` | same arguments |
| `boolean IsMatch2(String Pattern, int Options, String Text)` | same arguments |
| `String Replace(String Pattern, String Text, String Template)` | same arguments |
| `String Replace2(String Pattern, int Options, String Text, String Template)` | same arguments |
| `String[] Split(String Pattern, String Text)` | returns String() |
| `String[] Split2(String Pattern, int Options, String Text)` | returns String() |
| `MatcherWrapper Matcher(String Pattern, String Text)` | returns Matcher |
| `MatcherWrapper Matcher2(String Pattern, int Options, String Text)` | returns Matcher |

Rules:

- **source**, [Regex.java][regex-source]: non-`2` methods call their `2`
  variant with options 0. Patterns compile with `Pattern.compile` and are
  cached by pattern text **and** options. Each Matcher is fresh. Cache
  eviction is an optimization, not a change in pattern semantics.
- **source**, [Regex.java][regex-source]: `IsMatch2` invokes `matches`,
  `Replace2` invokes `replaceAll`, `Split2` invokes `split(Text)`.
  **forum**, [Erel on complete-string matches][forum-match], confirms
  that `IsMatch` is not a substring search.
- **docs (JDK 19)**, [Pattern][jdk-pattern]: Java regex syntax, embedded
  flags and Unicode rules apply. Options combine bitwise; CASE_INSENSITIVE
  alone is ASCII-oriented, while UNICODE_CASE changes case handling.
  MULTILINE changes anchors, not dot; **forum**, [Erel][forum-dotall],
  identifies numeric option `0x20` as DOTALL. Unknown flag bits raise
  `IllegalArgumentException`; invalid regex syntax raises
  `PatternSyntaxException` at Java compilation.
- **docs (JDK 19)**, [Matcher][jdk-matcher]: `matches` tests the entire
  input. Replacement templates interpret `$n`, `${name}` and backslash
  escapes; they are not literal replacement text. Invalid references can
  throw when replacement encounters a match.
- **docs (JDK 19)**, [Pattern.split][jdk-split]: the one-argument overload
  uses limit 0, dropping trailing empty elements. Capturing delimiters
  are not emitted. A positive-width match at the beginning produces a
  leading empty element; a zero-width match there does not.
- **source**, [Regex.java][regex-source]: there is no special exclusion
  of empty input in B4J's wrapper. Do not import the empty-input note from
  the [B4i reference](https://www.b4x.com/b4i/help/regularexpressions.html)
  as a B4J rule.

*(Open)* **L1L6**, compiler validation of invalid **literal** regexes:
the runtime source shows an exception but does not show whether the B4J
compiler checks literals first. Probe `level1/regex_invalid_literal`
distinguishes a build refusal from runtime `PatternSyntaxException`.
There is no open question about the documented successful method calls,
so no speculative probe duplicates those rules.

## Matcher members

Declarations: **source**, [Regex.MatcherWrapper, lines 146–199][matcher-source],
a subclass of `AbsObjectWrapper<Matcher>` with `@ShortName("Matcher")`.

| jCore declaration | B4J surface |
| --- | --- |
| `boolean Find()` | `Find As Boolean` |
| `String Group(int Index)` | `Group(Index As Int) As String` |
| `int getGroupCount()` | `GroupCount As Int`, read-only |
| `String getMatch()` | `Match As String`, read-only |
| `int GetStart(int Index)` | `GetStart(Index As Int) As Int` |
| `int GetEnd(int Index)` | `GetEnd(Index As Int) As Int` |
| inherited `boolean IsInitialized()` | `IsInitialized As Boolean` |

- **source**, [MatcherWrapper][matcher-source]: these delegate, respectively,
  to `find`, `group(Index)`, `groupCount`, `group()`, `start(Index)` and
  `end(Index)`. **docs**, [B4J reference][regex-docs]: group 0 is the whole
  match and is not counted in GroupCount; Match is Group(0).
- **docs (JDK 19)**, [Matcher][jdk-matcher]: Find advances to the next
  match; indices count UTF-16 units, start inclusive and end exclusive.
  An unmatched optional group returns null and offsets -1; an empty group
  returns an empty string. Reading match state before success or after
  failure raises `IllegalStateException`; invalid numeric group indices
  raise `IndexOutOfBoundsException`. GroupCount needs no successful match.
- **source**, [AbsObjectWrapper, lines 31–51][wrapper-source]: inherited
  IsInitialized tests the wrapped reference directly; it is false before
  a Matcher is assigned, true after Regex.Matcher/Matcher2. Other members
  call `getObject`, which raises RuntimeException if it is unset.

No additional open Matcher runtime rule was found. Generic wrapper
passing to the new keywords is **L1L4** below; no class or event is needed.

## DateTime parsing and formatting

Declarations: **source**, [DateTime.java, lines 44–218][date-source].
DateTime is predefined (**docs**, [Core reference][core]).

| jCore declaration | B4J surface |
| --- | --- |
| `String Date(long Ticks)` / `String Time(long Ticks)` | format a Long |
| `long DateParse(String Date) throws ParseException` | `DateParse(Date As String) As Long` |
| `long TimeParse(String Time) throws ParseException` | `TimeParse(Time As String) As Long` |
| `long DateTimeParse(String Date, String Time) throws ParseException` | `DateTimeParse(Date As String, Time As String) As Long` |
| `String getDateFormat()` / `void setDateFormat(String Pattern)` | read/write `DateFormat As String` |
| `String getTimeFormat()` / `void setTimeFormat(String Pattern)` | read/write `TimeFormat As String` |
| `String getDeviceDefaultDateFormat()` | read-only `DeviceDefaultDateFormat As String` |
| `String getDeviceDefaultTimeFormat()` | read-only `DeviceDefaultTimeFormat As String` |
| `long getNow()` | read-only `Now As Long` |
| `void SetTimeZone(double OffsetHours)` | `SetTimeZone(OffsetHours As Double)` |
| `double getTimeZoneOffset()` | read-only `TimeZoneOffset As Double` |
| `double GetTimeZoneOffsetAt(long Date)` | `GetTimeZoneOffsetAt(Date As Long) As Double` |

Rules:

- **docs**, [Core reference][core]: ticks are milliseconds since the UTC
  epoch, stored as Long. **source**, [DateTime.java][date-source]: initial
  patterns are `MM/dd/yyyy` and `HH:mm:ss`. Each thread has its own date
  state. Formatters use the default formatting locale and time zone;
  `Calendar.getInstance(Locale.US)` supplies the arithmetic/field calendar.
- **source**, [DateTime.java][date-source]: both SimpleDateFormats are
  non-lenient. Getters return `toPattern`, setters use `applyPattern`;
  Date/Time call `format(new Date(Ticks))` via a reused Date object.
- **docs (JDK 19)**, [SimpleDateFormat][jdk-sdf]: pattern letters are case
  sensitive; quoted text is literal. `MM` is month and `mm` minute; `yyyy`
  and `YYYY` are different year concepts. Invalid pattern letters raise
  IllegalArgumentException. Two-digit years depend on formatter creation
  time, so avoid them in machine-independent recordings.
- **source**, [DateParse][date-source]: calls `dateFormat.parse(Date)`.
  **docs (JDK 19)**, [DateFormat.parse][jdk-dateparse]: parsing may consume
  a prefix; strict calendar validity does not require full input
  consumption. A failure raises ParseException; numeric pattern widths
  generally do not impose exact input widths ([SimpleDateFormat][jdk-sdf]).
- **source**, [TimeParse, lines 152–165][date-source]: temporarily parses
  in a zero-offset zone, restores the application zone in `finally`, and
  adds the result modulo TicksPerDay to a day start computed from the
  current clock and current offset. **docs**, [Core reference][core]:
  the date part is today. It is not an epoch-only time parser.
- **source**, [DateTimeParse, lines 171–190][date-source]: temporarily sets
  both parsers to zero offset, sums their parsed milliseconds, then
  corrects for application-zone offsets at the sum and the adjusted sum.
  Both zones are restored in `finally`. It does not simply concatenate
  date/time text. Patterns containing their own date/zone fields still go
  through this algorithm; do not substitute a generic ISO parser.
- **source**, [SetTimeZone and offset methods][date-source]: SetTimeZone
  constructs a fixed SimpleTimeZone using rounded milliseconds, updating
  calendar and both formatters. OffsetAt uses the application zone for
  that instant; TimeZoneOffset uses the current clock. Defaults come from
  the machine. DeviceDefault patterns use Java DateFormat factories.

*(Open)* **L1L7**, whether an invalid literal pattern is rejected before
execution or raises at the setters: `level1/datetime_invalid_patterns`.
It uses no date value, clock or time zone. Runtime behavior is source-backed;
compiler validation is not established by these sources.

*(Open, waiting for OD5)* **L1L9**: the clock, default/time-zone setup,
DST gaps/overlaps, TimeParse's today and midnight boundary, default locale
patterns and the two-digit-year window need an owner-approved test contract.
No probe reads or sets the time zone, or reads Now in this batch.

## DateTime arithmetic and fields

Declarations: **source**, [DateTime.java, lines 222–297][date-fields].

| jCore declaration | B4J surface |
| --- | --- |
| `long Add(long Ticks, int Years, int Months, int Days)` | same arguments |
| `int GetYear(long Ticks)` | `GetYear(Ticks As Long) As Int` |
| `int GetMonth(long Ticks)` | same |
| `int GetDayOfMonth(long Ticks)` | same |
| `int GetDayOfYear(long Ticks)` | same |
| `int GetDayOfWeek(long Ticks)` | same |
| `int GetHour(long Ticks)` | same |
| `int GetMinute(long Ticks)` | same |
| `int GetSecond(long Ticks)` | same |
| `long TicksPerSecond = 1000L` | constant Long |
| `long TicksPerMinute = 60 * TicksPerSecond` | 60000 |
| `long TicksPerHour = TicksPerMinute * 60` | 3600000 |
| `long TicksPerDay = TicksPerHour * 24` | 86400000 |

- **source**, [field methods and Add][date-fields]: fields set the calendar
  instant and read the corresponding Calendar field; month adds 1 to
  Java's zero-based month. Add sets the instant, then calls Calendar.add
  on YEAR, MONTH, DAY_OF_YEAR in that order, even for zero deltas.
- **docs**, [Core reference][core]: months 1–12, days of week 1–7 with
  Sunday 1, day of year 1–366, hours 0–23, minutes/seconds 0–59. Negative
  Add arguments subtract.
- **docs (JDK 19)**, [Calendar.add][jdk-calendar]: calendar addition carries
  fields and clamps smaller fields when necessary (for example month-end).
  It is not fixed-duration arithmetic: one calendar day can differ from
  TicksPerDay across DST. **source**, [DateTime initialization][date-source]:
  the calendar is US-locale, in the application zone.

No unresolved ordinary arithmetic rule needs a new probe. Zone-sensitive
field/arithmetic recordings wait for **L1L9 / OD5**. Fixed ticks with a
numeric pattern alone do not fix a calendar field's zone; this batch does
not silently assume the recording machine uses UTC.

## Smart-string date/time formatters

Declaration: **source**, [Common.java, lines 791–802][smart-source]:
`String SmartStringFormatter(String Format, Object Value)`.
B4J syntax is `$"$date{expression}"$`, `$time{}` and `$datetime{}`.

- **docs**, [Language booklet §4.10.4][booklets]: date and time are
  equivalent to DateTime.Date/Time; datetime joins them with one space.
  Mixed-case `$DateTime{}` is shown in the booklet.
- **source**, [SmartStringFormatter][smart-source]: Format reaches the
  helper lowercased (the source's compiler contract comment); date and
  time call `BA.ObjectToLongNumber(Value)`. Datetime converts once and
  joins Date(l), a space and Time(l). It uses the current DateTime patterns,
  not a separate fixed format.
- **source**, [BA.ObjectToLongNumber][long-source]: Number uses longValue;
  other objects use Long.parseLong(String.valueOf(o)). Thus a decimal
  integer String can be converted; a fractional String cannot. This is a
  helper rule, not evidence that the compiler accepts every Object operand.

*(Open)* **L1L8**, binding an Object holding numeric text in these
formatters: `level1/smart_datetime_object`. Literal-only patterns `'D'`
and `'T'` make every answer independent of locale, clock and zone. A
compiler refusal, runtime error and successful helper coercion are distinct
outcomes. Ordinary typed-Long formatting is documented, so is not an open
question. DateTime's machine dependencies still wait for **L1L9 / OD5**.

## BytesToString and CharsToString

Declarations: **source**, [Common.java, lines 657–677][convert-source].

```java
String CharsToString(char[] Chars, int StartOffset, int Length)
String BytesToString(byte[] Data, @DefaultValue("0") int StartOffset,
    @DefaultValue("-1") int Length, @DefaultValue("UTF-8") String CharSet)
    throws UnsupportedEncodingException
```

- **docs**, [Core reference][core] and [Language booklet §4.2][booklets]:
  StartOffset is the first array element and Length is the count.
  CharsToString copies characters; BytesToString decodes a named charset.
- **source**, [CharsToString][convert-source]: `new String(Chars,
  StartOffset, Length)`. **docs (JDK 19)**, [String(char[], int, int)][jdk-chars]:
  copies UTF-16 units, without Unicode normalization or surrogate repair;
  invalid ranges throw IndexOutOfBoundsException, null arrays
  NullPointerException. No additional open CharsToString rule was found.
- **source**, [BytesToString][convert-source]: uses `new String(Data,
  StartOffset, Length == -1 ? Data.length : Length, CharSet)`.
  **forum**, [Erel's optional-parameter tutorial][forum-defaults], confirms
  omission of trailing arguments with defaults 0, -1, UTF-8. This task
  does not introduce optional parameters to FerroBASIC.
- **docs (JDK 19)**, [String(byte[], int, int, String)][jdk-bytes]: byte
  offsets/counts need not equal output character counts. Invalid ranges
  raise IndexOutOfBoundsException and unsupported charset names raise
  UnsupportedEncodingException. Malformed input behavior in this overload
  is unspecified; do not borrow the replacement guarantee of the distinct
  Charset-taking constructor.
- **docs**, [Language booklet §7.14][booklets]: byte-to-text conversion is
  for encoded text, not arbitrary binary serialization. That advice does
  not specify the observable result of a malformed sequence.

*(Open)* **L1L1**, Length=-1 at a nonzero offset: the source Javadoc says
all available bytes, but the expression passes the **whole array length**,
not the remainder. `level1/bytes_length_rest` distinguishes the source's
bounds exception from decoding the remaining `BC`.

*(Open)* **L1L2**, actual OpenJDK 19 decoding of malformed UTF-8 and
unmappable US-ASCII: `level1/bytes_invalid_sequences` logs UTF-16 numbers,
not replacement glyphs dependent on terminal encoding. Each call has its
own Catch so one exception cannot hide the other answer.

## Environment, system properties and product identity

Declarations: **source**, [Common.java, lines 409–410][tool-source] and
[688–703][environment-source].

```java
String GetEnvironmentVariable(String Key, @DefaultValue("") String DefaultValue)
String GetSystemProperty(String Key, @DefaultValue("") String DefaultValue)
void SetSystemProperty(String Key, String Value)
boolean IsDevTool(String ToolName)
```

- **source**, [Common.java][environment-source]: GetEnvironmentVariable
  returns System.getenv(Key), substituting DefaultValue only for null.
  GetSystemProperty forwards to System.getProperty(Key, DefaultValue).
  SetSystemProperty forwards to System.setProperty, discarding its return.
- **docs (JDK 19)**, [System][jdk-system]: properties and environment are
  different namespaces; setting a property does not set an environment
  variable. Empty property keys raise IllegalArgumentException; null keys
  or set values raise NullPointerException at the Java API. Environment
  name case behavior is platform dependent. Empty strings are values,
  not absence. Security checks can raise SecurityException.
- **source**, [IsDevTool][tool-source]: lowercases ToolName with `BA.cul`
  and compares it with `b4j`. Thus `B4J` and `b4j` are true; an unrelated
  name is false. This is product identity, not debug/release detection.
  No trimming occurs. **source**, [BA.cul][ba-source]: Locale.US.

No additional runtime question for IsDevTool or properties was found.
Their deterministic current diagnostics were checked using IsDevTool("B4J")
and a property explicitly set by the diagnostic sample itself. No probe
assumes even a long random-looking key is absent from the machine.

*(Open, waiting for OD5)* **L1L10**: an owner-approved environment variable,
missing-variable policy, initial system-property values and cross-platform
key case. There is no environment-reading conformance probe in this batch.
Setting a private property before reading it would be deterministic, but
these sources already settle that ordinary rule.

## Initialized and NotInitialized

Declarations: **source**, [Common.java, lines 606–655][initialized-source]:
`boolean Initialized(Object Object)` and `boolean NotInitialized(Object Object)`.
These are keywords, distinct from instance `IsInitialized`.
**forum**, [Erel's B4J 10.2 release][forum-initialized], establishes their
presence in B4J (not only in B4A/B4i).

- **source**, [Common.java][initialized-source]: Initialized is exactly
  `!NotInitialized(Object)`. NotInitialized returns true for null; for
  ObjectWrapper it tests getObjectOrNull for null; for B4AClass it negates
  IsInitialized. These branches precede reflection.
- **source**, [reflection branch][initialized-source]: for other classes,
  first attempts a public no-argument method named exactly IsInitialized,
  then, if its result is null, a public field with that exact name. A
  Boolean result is negated. No discoverable Boolean means non-null is
  initialized. Reflection exceptions are swallowed during discovery.
- **source**, [cache branch][initialized-source]: caches the discovered
  strategy by Java class (method 1, field 2, neither 0). It still reads
  each object's value on a successful strategy. Do not generalize the
  discovery fallback to every cached invocation: a cached read that fails
  can reach a null Boolean unboxing or an invalid cast.
- **forum**, [Erel, B4J release notes][forum-initialized]: Initialized(Map1)
  is offered as a shorthand for a null and initialization check.
  **forum (B4i)**, [Erel on primitive values][forum-primitives], says
  primitives are always initialized; **source**, [Common.java][initialized-source],
  is consistent with that when Java boxes them into non-null objects.

*(Open)* **L1L3**, how a typed null array reaches the keywords versus an
explicitly allocated empty array: `level1/initialized_array`. The null array
is assigned through Object, as in recorded Level 0 `list_from_null_array`;
this does not reopen the settled default of `Dim U() As Int` (empty array).

*(Open)* **L1L4**, whether an uninitialized List passed directly or through
Object preserves the wrapper's state without raising:
`level1/initialized_wrapped_object`. The release example is not generated
Java for these two argument paths.

*(Open)* **L1L5**, reflection of the generated Type's IsInitialized field
when passed as Object, before and after Initialize:
`level1/initialized_record`. Existing Level 0 recordings establish the
field exists (e.g. `list_sort_type_initialized`), but do not call these new
keywords. User-defined classes and their initialization remain task D's
multi-module questions; no class module is added here.

## Probe inventory and current diagnostics

Each row is a question, not an expected-output file. No finite table can
enumerate arbitrary unexpected stdout: numeric unit logging and explicit
error/accepted labels preserve any unanticipated answer for owner review.
The candidate answers below are the hypotheses distinguished by each probe.
All eight have `level = 1`, `status = "pending-recording"`,
`ferro_rejects = true`, and were listed by today's recorder dry run.

| Question / probe under `level1/` | Candidate recorded stdout (lines separated by `;`) | FerroBASIC today |
| --- | --- | --- |
| L1L1 `bytes_length_rest` | Whole-array length: `bounds=true`; remainder: `text=BC`; other error: `bounds=false` | refuses BytesToString (Level 1) |
| L1L2 `bytes_invalid_sequences` | Replacement: `utf8=65533,40; ascii=65533`; dropping: `utf8=40; ascii=`; exceptions: `utf8=error` / `ascii=error`; other policies print their UTF-16 units | refuses BytesToString (Level 1) |
| L1L3 `initialized_array` | References preserved: `null=false,true; empty=true,false`; all typed arrays considered initialized: `null=true,false`; runtime conversion failure: corresponding `=error`; compiler refusal: no stdout | refuses both keyword names as undeclared |
| L1L4 `initialized_wrapped_object` | State preserved: `direct=false,true; boxed=false,true`; lost state: corresponding `true,false`; eager unwrapping error: `direct=error` / `boxed=error`; build refusal: no stdout | refuses both keyword names as undeclared |
| L1L5 `initialized_record` | Field observed: `unset=false,true; ready=true,false`; only null checked: `unset=true,false; ready=true,false`; runtime errors: label `=error`; build refusal: no stdout | refuses both keyword names as undeclared |
| L1L6 `regex_invalid_literal` | Runtime validation: `pattern=true`; compile-time validation: no stdout, build refusal; no validation: `false; accepted` (or `true; accepted`); other error: `pattern=false` | refuses Regex (Level 1) |
| L1L7 `datetime_invalid_patterns` | Runtime validation: `date=true; time=true`; compile-time validation: no stdout, build refusal; accepted setter: `date=accepted` / `time=accepted`; other error: label `=false` | refuses DateTime (Level 1) |
| L1L8 `smart_datetime_object` | Object coercion: `d=true; t=true; dt=true`; runtime error: `error` (possibly after prior lines); different semantics: corresponding `false`; compile-time refusal: no stdout | refuses DateTime and all three formatters (Level 1) |

`ferro check` exits 3 for every probe. Temporary diagnostic
samples checked 56 member/property/formatter cases across every signature group, including Matcher2,
Matcher members, CharsToString, property reads/writes and IsDevTool.
Regex/DateTime accesses report the object's Level 1 diagnostic; byte/char
conversions report Level 1 conversions; environment/property/IsDevTool calls
report Level 1 process environment. A declared Matcher is an unknown type;
Initialized/NotInitialized are undeclared names. These existing diagnostics
are evidence of today's refusal, not proposed B4J diagnostics. No checker,
recorder or interpreter code changes are part of this task.

**L1L9** and **L1L10** have no probe until OD5. This respects OD6's assigned
library research location without moving language rules out of the draft spec.

[jcore]: https://github.com/AnywhereSoftware/B4J/tree/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore
[core]: https://www.b4x.com/b4j/help/core.html
[regex-docs]: https://www.b4x.com/b4j/help/regularexpressions.html
[booklets]: https://www.b4x.com/android/files/Booklets.zip
[regex-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Regex.java#L41-L132
[matcher-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Regex.java#L146-L199
[wrapper-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/AbsObjectWrapper.java#L31-L51
[date-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/DateTime.java#L44-L218
[date-fields]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/DateTime.java#L222-L297
[smart-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L791-L802
[long-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L443-L450
[convert-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L657-L677
[environment-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L688-L703
[tool-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L406-L411
[ba-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java
[initialized-source]: https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L606-L655
[jdk-pattern]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/util/regex/Pattern.html
[jdk-split]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/util/regex/Pattern.html#split(java.lang.CharSequence)
[jdk-matcher]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/util/regex/Matcher.html
[jdk-sdf]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/text/SimpleDateFormat.html
[jdk-dateparse]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/text/DateFormat.html#parse(java.lang.String)
[jdk-calendar]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/util/Calendar.html#add(int,int)
[jdk-bytes]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/lang/String.html#%3Cinit%3E(byte%5B%5D,int,int,java.lang.String)
[jdk-chars]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/lang/String.html#%3Cinit%3E(char%5B%5D,int,int)
[jdk-system]: https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/lang/System.html
[forum-match]: https://www.b4x.com/android/forum/threads/strange-behavior-regex.109929/
[forum-dotall]: https://www.b4x.com/android/forum/threads/need-help-with-regex-and-replacing-stings-in-a-html-file.28268/
[forum-defaults]: https://www.b4x.com/android/forum/threads/b4x-optional-sub-parameters.171906/
[forum-initialized]: https://www.b4x.com/android/forum/threads/b4j-change-log-version-history.37448/
[forum-primitives]: https://www.b4x.com/android/forum/threads/b4i-v8-90-beta-has-been-released.166916/
