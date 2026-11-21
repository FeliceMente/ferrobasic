# Language specification — Level 1 and Level 2 (draft)

**Draft, November 2026.** Nothing here is implemented yet, and nothing here
has been recorded. This page states what the public sources say about what
Levels 1 and 2 add to [Level 0](level0.md), so that the M2 work
([roadmap](../roadmap.md#m2-task-breakdown)) has a plan and the first probes
have questions to answer. A recording always overrides it.

- Level 1: several modules, code modules and classes (`Class_Globals`,
  `Initialize`, `Me`, `IsInitialized`), name-bound events, `Sender`,
  `Timer` and the message loop.
- Level 2: resumable subs (`Sleep`, `Wait For`, `ResumableSub`),
  `CallSub*`, `CallSubDelayed*` and `SubExists`.

Everything Level 0 says still holds, inside every module. The library
members Level 1 adds (`DateTime`, `Regex`, `BytesToString`,
`CharsToString`, `GetEnvironmentVariable` and friends) are researched
separately (task C of
[#146](https://github.com/FeliceMente/ferrobasic/issues/146)) and are not
specified here.

## Sources and labels

Every rule says where it comes from, with the labels of
[research/level0-semantics.md](../research/level0-semantics.md#sources):

- **source**: read in the open-source jCore runtime, Apache-2.0,
  [AnywhereSoftware/B4J](https://github.com/AnywhereSoftware/B4J) at commit
  `e26e645a611dd24c2034fbbeaf11ba26b70dee8b` (its `Common` is marked
  `@Version(10.70f)`, the version the corpus is recorded with). It settles
  what the runtime does once it is called; it does not say how the compiler
  calls it.
- **generated**: seen in Java produced by the B4X compiler, in the
  `Objects/src` folders of the example projects in
  [Booklets.zip](https://www.b4x.com/android/files/Booklets.zip). The only
  such code for this level is **B4A**'s
  (`B4R/B4RExamplesSourceCode/HC05/B4A/New/HC05LedOnOff/Objects/src/b4a/HC05LedOnOff/`,
  `main.java` and the class `bluetoothasynchstream.java`), so these rules
  are labelled **generated (B4A)**: B4J's compiler may differ, and each
  one is also a question for a probe. Used locally, never committed.
- **docs**: the *B4X Language* booklet v2.5 and the *B4X CustomViews*
  booklet v2.5, both in
  [Booklets.zip](https://www.b4x.com/android/files/Booklets.zip), cited by
  section (`Language §4.7.3`), and the jCore Javadoc that the
  [Core reference](https://www.b4x.com/b4j/help/core.html) is built from.
- **forum**: answers by Erel (Anywhere Software) on the B4X forum, linked
  per rule. Some are tutorials written for B4A; Erel's
  [B4J tutorial for B4A developers](https://www.b4x.com/android/forum/threads/b4j-tutorial-for-basic4android-developers.34659/#post-203112)
  says B4J's classes "are identical to B4A classes", so those apply to B4J
  classes, and are marked "(B4A tutorial)".
- *(open)*: no source settles it, or the sources disagree. It is a
  question for a probe, listed in [Questions for probes](#questions-for-probes)
  by its number (Q1, Q2, …).

As in [level0.md](level0.md), a rule marked *(open)* is implemented as
stated (usually as a refusal with a "not known yet" diagnostic), kept in
one place so a recording can change it, and tested only by pending tests.

jCore links are to files under
`https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/`,
listed at the [end of the page](#jcore-references).

## Modules

### Kinds of module

- A project has one main module and any number of other modules, each a
  `.bas` file named in the project file
  ([project-files.md](project-files.md)). A module's header says its kind:
  `Type=StaticCode` for a code module, `Type=Class` for a class (Level 0's
  reader already reads both; `Type=Class` is refused today).
- In B4J there are two kinds of module, code modules and classes, and
  "code modules are similar to B4A modules however they are more powerful
  as they can handle events. A code module can be considered as a class
  with exactly one instance" (forum:
  [B4J tutorial for B4A developers](https://www.b4x.com/android/forum/threads/b4j-tutorial-for-basic4android-developers.34659/#post-203112)).
  B4A's limitation that code modules cannot handle events "is not the case
  in B4J and B4i" (forum:
  [Erel](https://www.b4x.com/android/forum/threads/allow-wait-for-in-code-modules.116499/#post-727884)).
  The B4X Language booklet says the opposite, for every product: "it is
  impossible to handle events or use CallSub with code modules"
  (docs: Language §4.16.1). The forum answers are B4J-specific and later,
  so FerroBASIC follows them, but which events a B4J code module handles
  is *(open)* (Q14, Q30).
- The main module of a B4J console program is a code module: the Level 0
  program (spec: [level0.md](level0.md#program-structure)).

### Process_Globals of every module

- Every code module may have a `Sub Process_Globals`. "This sub is called
  once when the process starts (this is true for all modules, not just the
  main module)" (docs: Language §2.6.1). Whether a module other than the
  main one must have one, as the main module must, is *(open)* (Q3).
- When each module's `Process_Globals` runs, relative to the others and to
  `AppStart`, is *(open)* (Q1). B4A's generated code runs the main module's
  first, then each other module's, in a fixed sequence, before the first
  event (generated (B4A): `main.initializeProcessGlobals`). Whether a
  module's `Process_Globals` reading another module's variable sees it
  initialised is *(open)* (Q2).
- Process variables are the only variables other modules can reach. `Dim`
  and `Public` declare a public one; `Private` one that only its own module
  sees (docs: Language §2.6.1; forum:
  [Variables and subs visibility](https://www.b4x.com/android/forum/threads/variables-and-subs-visibility.18628/#post-106839),
  B4A tutorial). Reading or assigning another module's private variable is
  a compile error; its message is *(open)* (Q5).

### Names across modules

- Another module's process variable is written with its module's name as
  a prefix: `MyModule.MyVar = "Text"`; inside `MyModule` it is `MyVar`
  (docs: Language §2.6.1).
- A Sub of a code module is called from another module with the module's
  name as a prefix, `MyModule.CalcInterest(1234, 5.2)`, "instead of"
  `CalcInterest(1234, 5.2)` (docs: Language §4.6.3). What B4J does with the
  unprefixed call, when no Sub of the calling module has that name, is
  *(open)* (Q4); FerroBASIC refuses it meanwhile.
- Subs of code modules and classes are public by default; `Private Sub`
  hides one from other modules (forum:
  [Variables and subs visibility](https://www.b4x.com/android/forum/threads/variables-and-subs-visibility.18628/#post-106839),
  B4A tutorial). Calling another module's private Sub directly is a
  compile error *(open: the message, Q5)*.
- Two Subs of one module cannot share a name (docs: Language §4.6.4).
  Whether two modules may each have a Sub `F` is *(open)* (Q6): the prefix
  of §4.6.3 suggests that each module is its own namespace, but no source
  says so.
- Names are case-insensitive (docs: Language §2.2), module names included
  *(open: Q6 checks `module2.F` against `Module2`)*.
- What a module's name means as a value (`Log(Module2)`, passing it as an
  argument) is *(open)* (Q30). It is passed as the `Component` of
  `CallSub*`, `CallSubDelayed*` and `SubExists` (docs: Language §4.2,
  `CallSub(Main, "RefreshData")`).
- A local variable, a module variable or a parameter named like a module,
  and a module named like a `Type`, a Sub or a keyword, are *(open)*;
  FerroBASIC refuses them meanwhile (Q7).
- A module's `Type`s are used from another module as
  `MyModule.MyType`, or unprefixed: *(open)* (Q8).

## Classes

### Structure

- A class module has `Sub Class_Globals` and `Sub Initialize` (docs:
  Language §4.16.2.1; forum:
  [Classes tutorial](https://www.b4x.com/android/forum/threads/classes-tutorial.18626/#post-106825),
  B4A tutorial: "Classes must have the following two subs"). What B4J does
  with a class missing either is *(open)* (Q9).
- `Class_Globals` declares the instance variables. `Public`, `Private` and
  `Dim` follow the rules of `Process_Globals`: `Dim` is public, as `Public`
  is (docs: Language §2.6.1). Other modules read and write a public one as
  `instance.Name`.
- `Initialize` may take any parameters: "you can modify this sub signature
  and add arguments as needed" (docs: Language §4.16.2.1).
- Each instance has its own copy of the `Class_Globals` variables
  (docs: Language §4.16.1: classes are templates, "you can instantiate any
  number of objects").
- A class's Subs are called on an instance, `p.GetName`; inside the class,
  directly, `GetName`. A `Private Sub` of a class cannot be called from
  outside it (forum:
  [Variables and subs visibility](https://www.b4x.com/android/forum/threads/variables-and-subs-visibility.18628/#post-106839),
  B4A tutorial).
- Whether a class may also have a `Process_Globals` is *(open)* (Q9).
- `Class_Globals` follows Level 0's rules for `Process_Globals` where the
  sources say so; the open points Level 0 left for it
  ([level0.md](level0.md#program-structure): a `Dim` with a Sub's name, and
  loops, which must use a variable declared with `Dim`) stay *(open)*
  (Q10).

### Properties

Classes support properties: one or two Subs named `get<Prop>` and
`set<Prop>`, with `get` and `set` in lower case, are a property `Prop` that
other modules read and assign like a field: `c1.Text = "abc"`,
`Log(c1.Text)`. A property may be read-only or write-only; the get Sub's
return type and the set Sub's parameter type must be the same; inside the
class the Subs are called directly; a property cannot have a global
variable's name (forum:
[Classes tutorial](https://www.b4x.com/android/forum/threads/classes-tutorial.18626/#post-106825),
"Properties", B4A tutorial). What B4J does when the rules are broken is
*(open)* (Q11).

### Instances, `Initialize` and `IsInitialized`

- `Dim p As Person` gives `p` a new, uninitialised instance: B4A compiles
  it to `new bluetoothasynchstream()` (generated (B4A)). The instance has
  no context yet: jCore's `IsInitialized` on a class instance is
  `ba != null` (source: [`B4AClass.ImplB4AClass`](#jcore-references)).
- `p.Initialize(…)` first gives the instance its context, then runs
  `Class_Globals`, then `Initialize`'s own body: B4A compiles `Initialize`
  to `innerInitialize(_ba)` followed by the body, and `innerInitialize`
  raises `class_globals` (generated (B4A)). So the `Class_Globals`
  variables hold their declared values only once `Initialize` has been
  called, and what they hold before is *(open)* (Q12).
- `innerInitialize` creates the context only once, but runs
  `Class_Globals` at every call (generated (B4A)): calling `Initialize`
  again on the same instance resets its variables. *(open for B4J, Q13.)*
- "A class object must be initialized before you can call any other sub"
  (docs: Language §4.16.2.1). What calling one on an uninitialised
  instance does is *(open)* (Q12).
- Instances are references: after `p2 = p`, "both variables now point to
  the same Person object", and `p2` needs no `Initialize` (docs: Language
  §4.16.2.1).
- "Classes store a reference to the context of the module that called the
  Initialize sub" (docs: Language §4.16.1). The events of the objects a
  class initialises go to the class: in B4A, `Admin.Initialize("Admin")`
  inside a class compiles to `_admin.Initialize(ba, "Admin")`, with the
  class's own context (generated (B4A)).
- `p.IsInitialized` is `True` once `Initialize` has run (source:
  `ImplB4AClass.IsInitialized`). jCore 10.70 also has the keywords
  `Initialized(o)` and `NotInitialized(o)`, which test a class instance the
  same way and are `False`/`True` for `Null` (source:
  [`Common.NotInitialized`](#jcore-references)); they belong to the Level 1
  library work (task C).
- `Log(p)` and `p` converted to a String give `BA.TypeToString(p, true)`:
  `[name=value, name=value, …]`, one entry per instance variable, whose
  name is its Java field's without the leading `_` (so lower case, if the
  compiler lower-cases it as B4A does), with a line break after every third
  entry (source: [`BA.TypeToString`](#jcore-references), called by
  `ImplB4AClass.toString`). The field order (Java's `getDeclaredFields`,
  unspecified by the JDK) and the exact text are *(open)* (Q15).
- `GetType(p)` is the instance's Java class name (source:
  [`Common.GetType`](#jcore-references)); B4A names a class's Java class
  after the module in lower case, in the project's package (generated
  (B4A)). B4J's name is *(open)* (Q15).
- `=` and `<>` on two instances, and an instance against `Null`, are
  *(open)* (Q16).
- Polymorphism is duck typing: an `Object` holding any instance can be
  called with `CallSub*` by Sub name, and `SubExists` tests for a Sub
  (docs: Language §4.16.1.2). `x Is Person` tests a class (docs: the
  same); type tests are not part of any level yet
  ([level0.md](level0.md#program-structure)).

### Me

- "For classes: returns a reference to the current instance. For
  activities and services: returns a reference to an object that can be
  used with CallSub, CallSubDelayed and SubExists keywords. Cannot be used
  in code modules." (docs: jCore Javadoc of
  [`Common.Me`](#jcore-references), and Language §4.2 and §4.16.1.3.)
- In B4J the main module of a console program uses `Me` all the same:
  Erel's console tutorial calls `j.Initialize("j", Me)` in `AppStart`
  (forum: [Non-UI applications](https://www.b4x.com/android/forum/threads/non-ui-applications.34657/#post-203099)),
  and the booklet's resumable-sub example calls
  `CallSubDelayed(Me, "SecondSub_Complete")` (docs: Language §4.7.4). The
  sources disagree for code modules, so what `Me` is in the main module
  and in another code module of B4J is *(open)* (Q14).
- Inside a class, `Me` is the instance: `Shapes.Add(Me)` adds this object
  (docs: Language §4.16.1.3). Whether `Me.Name` reaches the class's own
  variable or Sub is *(open)* (Q16).

## Events

### Name-bound event Subs

- An object that raises events is initialised with an event name (a
  prefix). Its event `Ev` calls the Sub `<prefix>_<Ev>` of the module or
  instance that called `Initialize` (docs: Language §4.12, §4.16.1;
  source: [`Timer.Initialize`](#jcore-references) keeps
  `EventName.toLowerCase(Locale.US) + "_tick"`).
- The binding is by name and case-insensitive: jCore looks the Sub up in a
  table of the module's Subs keyed by their lower-cased names, and the
  event name is lower-cased too (source:
  [`BA.loadHtSubs`](#jcore-references), [`BA.raiseEvent2`](#jcore-references)).
  It is done at run time, when the event is raised; FerroBASIC may resolve
  it at compile time when it can
  ([architecture.md](../architecture.md#semantics-summary)), with the same
  result.
- When there is no such Sub, `raiseEvent2` does nothing, unless the caller
  asks for an error: then it raises `Exception: Sub <name> was not found.`
  (source). `Timer` asks for one (source: `Timer.TickTack`); `CallSub*` on
  a class instance asks for one (source: `Common.CallSub4`). Whether B4J's
  compiler refuses a `Timer` with no `_Tick` Sub before that is *(open)*
  (Q19).
- When the Sub's parameters do not match, `raiseEvent2` raises
  `Exception: Sub <name> signature does not match expected signature.`
  (source). Whether B4J's compiler checks an event Sub's signature first is
  *(open)* (Q20).
- A class raises its own events by calling a Sub of the object that
  initialised it: it keeps the caller (passed as `Me`) and the event name,
  and calls `CallSub*(mCallback, mEventName & "_Ev", …)`, or
  `CallSubDelayed*` (docs: CustomViews §4.2.1 and §4.5;
  generated (B4A): `CallSubDelayed2(mParent, mEventName & "_NewText", …)`).
  `#Event:` and `#RaisesSynchronousEvents:` lines declare the events for
  the IDE ("important for intellisense") (docs: CustomViews §4.2.1); that
  they change nothing at run time is *(open)* (Q21).

### Sender

- `Sender` "returns the object that raised the event. Only valid while
  inside the event sub and before calls to Sleep or Wait For" (docs: jCore
  Javadoc of [`Common.Sender`](#jcore-references); Language §4.2 without
  the last clause).
- jCore keeps it in a thread-local that `raiseEvent2` sets to the sender
  before calling the Sub and sets to `Null` when the Sub returns, without
  restoring the previous one (source: [`BA.raiseEvent2`](#jcore-references)).
  For a `Timer`, the sender is the `Timer` object itself (source:
  `Timer.TickTack`). For a Sub called by `CallSub*`, it is the calling
  module's or instance's events target (source:
  [`Common.CallSub4`](#jcore-references)).
- So, by the source, `Sender` read in an event Sub after a nested
  `CallSub` is `Null`, and in a Sub called directly from an event Sub it is
  the event's sender. Both depend on how B4J compiles `Sender` and Sub
  calls, so they are *(open)* until recorded (Q22), as is `Sender` outside
  any event (Q22) and after a `Sleep` (Q23).

### Timer

- `Timer.Initialize(EventName, Interval)` with `Interval` in milliseconds
  (a `Long`); `Enabled`, `False` by default; `Interval`; one event, `Tick`
  (docs: Language §4.12; source: [`Timer`](#jcore-references)).
- Setting `Enabled = True` with an `Interval` of 0 or less raises
  `IllegalStateException: Interval must be larger than 0.` (source:
  `Timer.setEnabled`). Setting `Enabled` to the value it has does nothing
  (source).
- Each enabled timer has a daemon thread that sleeps `Interval`
  milliseconds, then posts a tick to the message loop of the module that
  initialised it, and repeats; ticks only run when the loop runs them, so a
  busy main thread delays them (source: `Timer.TickTack`; docs: Language
  §4.12: "timer events will not fire while the UI thread is busy").
  Disabling, or changing `Interval` while enabled, discards the ticks
  already posted (source: the `relevantTimer` counter).
- `IsInitialized` is `True` after `Initialize` (source:
  `Timer.IsInitialized`).
- The booklet's B4J example writes `Timer1.Initialize("Timer1_Tick", 1000)`
  for a Sub `Timer1_Tick` (docs: Language §4.12), which by the source would
  look for `timer1_tick_tick`. The source settles it: the event name is the
  prefix only.

### The message loop

- A console program "will end after this sub [`AppStart`] unless you call
  StartMessageLoop. Calling StartMessageLoop means that the main thread is
  now handling an internal message queue" (forum:
  [Non-UI applications](https://www.b4x.com/android/forum/threads/non-ui-applications.34657/#post-203099)).
- `StartMessageLoop` runs the messages of the main thread's queue in the
  order they were posted, until it takes the stop message; then it returns,
  and the statement after it runs (source:
  [`SimpleMessageLoop.runMessageLoop`](#jcore-references),
  [`StandardBA.startMessageLoop`](#jcore-references)).
  Called from another thread it raises
  `RuntimeException: StartMessageLoop called from wrong thread.` (source).
- `StopMessageLoop` posts the stop message at the end of the queue: what
  was posted before it still runs, what is posted after it does not (source:
  `StandardBA.stopMessageLoop`). Called with no loop running, the stop
  message waits in the queue for the next `StartMessageLoop`, which returns
  at once (source); *(open)* until recorded (Q24).
- Without `ExitApplication` or `StopMessageLoop`, a program whose loop has
  nothing left to do "will just hang" (forum:
  [Non-UI applications](https://www.b4x.com/android/forum/threads/non-ui-applications.34657/#post-203099)).
  FerroBASIC does the same: it never ends a program B4J would leave
  running. A test of such a program is impossible to record; the
  conformance tests always stop their loop.
- When `AppStart` returns, the program ends: the threads of timers and of
  `Sleep` are daemon threads (source: `Timer.startTicking`,
  [`Common.Sleep`](#jcore-references); forum: Non-UI applications). The
  exit code is 0 *(open, as at Level 0)*.
- An error that a Sub run from the loop does not catch ends the program:
  `raiseEvent2` prints it and calls `ExitApplication2(1)` (source:
  [`BA.raiseEvent2`](#jcore-references)), unless the main module has
  `Application_Error (Error As Exception, StackTrace As String) As Boolean`,
  which is called first: returning `False` ignores the error, `True` lets it
  end the program (source: `BA.handleUncaughtException`; docs: Language
  §3.4 shows the Sub in the UI template). Whether a console program's
  `Application_Error` is called is *(open)* (Q25).

## Resumable subs (Level 2)

### Sleep and Wait For

- "Any sub with one or more calls to Sleep or Wait For is a resumable sub"
  (docs: Language §4.7; forum:
  [Resumable subs](https://www.b4x.com/android/forum/threads/b4x-resumable-subs-sleep-wait-for.78601/#post-498072)).
  It is "a variant of stackless coroutines" (same sources): FerroBASIC
  compiles it to a state machine with a heap frame
  ([architecture.md](../architecture.md#semantics-summary)), as B4A does
  (generated (B4A): a `ResumableSub_<name>` class whose `resume` method is a
  `switch` on `state`, with the Sub's locals and parameters as fields).
- "Whenever Sleep or Wait For are called, the current sub is paused. This
  is equivalent to calling Return": the caller continues at once (docs:
  Language §4.7.3; forum: Resumable subs). The booklet's example prints
  `S1: A`, `S2: A`, `S1: B`, `S2: B`.
- `Sleep(ms)` resumes the Sub after `ms` milliseconds, through the message
  loop: a scheduler thread waits, then posts the resumption to the loop of
  the Sub's module (source: [`Common.Sleep`](#jcore-references)).
  `Sleep(0)` is "the shortest pause" (docs: Language §4.7.1).
- "Each call to a resumable sub creates a different instance which is not
  affected by other calls" (forum: Resumable subs); its locals keep their
  values across the pause (generated (B4A): they are fields of the
  instance).
- `Wait For <prefix>_<Event> (params)` pauses until the event is raised,
  then binds its parameters and continues; "if the event is never raised
  then the sub will never be resumed" (docs: Language §4.7.2). The event
  name is lower-cased, and the parameters are taken from the event's
  arguments by position and cast to their declared types (generated (B4A):
  `Common.WaitFor("activity_permissionresult", …)`, then
  `_permission = (String) result[0]`). What a mismatched argument type does
  is *(open)* (Q29).
- `Wait For (<sender>) <event>` waits only for the event raised by that
  sender, compared by identity (docs: Language §4.7.2; source:
  [`BA.checkAndRunWaitForEvent`](#jcore-references)).
- A `Wait For` of an event that another paused instance waits for, with
  the same sender filter or none, replaces it: the earlier one is never
  resumed (docs: Language §4.7.2; source: [`Common.WaitFor`](#jcore-references)).
  Waits with a sender filter are checked before those without (source).
- "Wait For events handlers precede the regular event handlers" (docs:
  Language §4.7.10): when a waiting Sub takes an event, the event's own Sub
  is not called (source: `raiseEvent2` returns after
  `checkAndRunWaitForEvent`). The event can come from a library object,
  `CallSub*` or `CallSubDelayed*` (source: all go through `raiseEvent2`).
- Resumable subs rely on the message loop: "there is no message queue in
  console apps, unless we call StartMessageLoop", so a `Sleep` or `Wait For`
  in `AppStart` ends `AppStart`, and the program, there; the pause belongs
  in a Sub that `AppStart` calls before `StartMessageLoop` (forum:
  [Erel, 2026](https://www.b4x.com/android/forum/threads/resumable-subs-in-console-non-ui-apps.171462/#post-1048618)).
- Resumable subs "do not create additional threads": the main thread runs
  them (docs: Language §4.7.10).
- An error raised after a resumption is not caught by the caller, which has
  returned: the loop's runnable rethrows it wrapped in a `RuntimeException`
  (source: `Common.Sleep`), and what then happens to the program, and to a
  `Try` around `StartMessageLoop`, is *(open)* (Q27). A `Try` whose block
  contains a `Sleep` is *(open)* (Q28): jCore's `ResumableSub` has a
  `catchState` field for it (source: [`BA.ResumableSub`](#jcore-references)).
- The order of resumptions due at nearly the same time (a `Sleep(0)`
  against a `CallSubDelayed` posted just after it, two timers) depends on
  the scheduler thread, so it is not deterministic in B4J (source: the
  scheduler posts to the loop from its own thread). Conformance tests must
  not depend on it; how FerroBASIC orders them is an owner decision
  ([roadmap](../roadmap.md#m2-owner-decisions)).

### Returning a value: ResumableSub

- A resumable sub cannot simply return a value, since its caller has
  continued (docs: Language §4.7.5; forum:
  [ResumableSub](https://www.b4x.com/android/forum/threads/b4x-resumable-subs-that-return-values-resumablesub.82670/#post-523409)).
  What B4J does with a resumable sub declared with another return type is
  *(open)* (Q26).
- Declared `As ResumableSub`, it returns a `ResumableSub` object to its
  caller at its first pause; its `Return x` completes it. A caller waits for
  it with `Wait For (rs) Complete (Result As T)`, or in one line
  `Wait For (Sum(1, 2)) Complete (Result As Int)` (docs: Language §4.7.5;
  forum: ResumableSub). The `Result` name is free; "if you don't need to
  return a value … return Null … and set the type in the calling sub to
  Object"; "you can wait for resumable subs in other modules" (same).
- The completion is delivered through the message loop: `Return` posts it,
  and it raises `Complete` with the `ResumableSub` as sender (source:
  [`Common.ReturnFromResumableSub`](#jcore-references)). So a caller that
  calls `Wait For (rs) Complete` right after the call receives it even when
  the Sub returned without pausing *(open until recorded, Q26)*.
- `rs.Completed` is `True` once it has completed (source:
  `ResumableSubWrapper.getCompleted`). `Wait For (rs)` on a completed one
  raises `RuntimeException: Resumable sub already completed` (source:
  `Common.WaitFor`).

### CallSub, CallSubDelayed and SubExists

- `CallSub(Component, Sub)`, `CallSub2(Component, Sub, Arg)` and
  `CallSub3(Component, Sub, Arg1, Arg2)` call a Sub by its name, at once,
  and return its value as an `Object` (docs: jCore Javadoc and Language
  §4.2; source: [`Common.CallSub4`](#jcore-references)). The name is
  case-insensitive (source: lower-cased). "CallSub is mostly not required
  in B4J as you can directly call methods of other classes or code modules"
  (forum: [B4J tutorial for B4A developers](https://www.b4x.com/android/forum/threads/b4j-tutorial-for-basic4android-developers.34659/#post-203112)).
- `Component` is an instance, a module, or `Me` (docs: Language §4.2,
  §4.16.1.2). On a class instance, a missing Sub raises
  `Sub <name> was not found.`; on anything else it returns `Null` (source:
  `CallSub4` asks for the error only for a class). On an uninitialised
  instance it calls nothing and returns `Null`, and `Null` or `""` as
  `Component` means the calling module (source:
  [`Common.getComponentBA`](#jcore-references)). What the compiler passes
  for a module's name, and so what `CallSub` on a code module does, is
  *(open)* (Q30, Q31).
- `CallSub*` can call private Subs as well as public ones (forum:
  [Variables and subs visibility](https://www.b4x.com/android/forum/threads/variables-and-subs-visibility.18628/#post-106839),
  B4A tutorial; source: the table holds every Sub). *(open for B4J, Q31.)*
- An error raised in the called Sub: `raiseEvent2` wraps it in a
  `RuntimeException`, and when no other event is running it prints it and
  ends the program with exit code 1 (source: `raiseEvent2`'s `nestLevel`).
  Whether `AppStart` itself counts as a running event, so that a `Try`
  around a `CallSub` in `AppStart` catches the error, is *(open)* (Q32).
- `CallSubDelayed(Component, Sub)`, `CallSubDelayed2` and
  `CallSubDelayed3` post the call to the message loop and return at once;
  "the sub will be called after the current code execution completes"
  (docs: jCore Javadoc; source:
  [`Common.CallSubDelayed4`](#jcore-references)). Posted calls run in the
  order they were posted (source: a FIFO queue). On an uninitialised
  instance it fails at once, as `getComponentBA` gives no context (source);
  the error is *(open)* (Q33).
- "It is safer to use CallSubDelayed than CallSub" to signal the end of a
  resumable sub: `CallSub` "will fail if the second sub is never paused"
  (docs: Language §4.7.4).
- `SubExists(Object, Sub)` "tests whether the object includes the
  specified method. Returns false if the object was not initialized or not
  an instance of a user class" (docs: jCore Javadoc and Language §4.2). It is
  case-insensitive and `False` for `Null` (source:
  [`Common.SubExists`](#jcore-references)). On a module name it is
  *(open)* (Q30).

## What FerroBASIC refuses meanwhile

As at Level 0, every construct whose behaviour is *(open)* gets a precise
"not known yet" diagnostic until a recording settles it, and a ❌ or 🟡 row
in [language-support.md](../language-support.md). In particular, until
their probes are recorded: a module's name used as a value, `Me` in a code
module other than the main one, `Sender` outside an event Sub, a `Try`
around a `Sleep` or `Wait For`, a resumable sub with a return type other
than `ResumableSub`, and an event Sub whose signature does not match the
event.

## Questions for probes

Each question is meant to be answered by one small program's printed
output (task D of [#146](https://github.com/FeliceMente/ferrobasic/issues/146)
turns them into pending tests). "Module2" is a code module, "Person" a
class, unless the question says otherwise. Programs that use the message
loop end it with `StopMessageLoop` or `ExitApplication`, so every probe
terminates.

### Modules

1. **Order of `Process_Globals`.** Main and Module2 each `Log` a label in
   `Process_Globals`, and `AppStart` logs one: in what order do the three
   lines print? Does Module2's run at all when nothing uses Module2?
2. **Cross-module initialisation.** Module2's `Process_Globals` logs
   `Main.X`, declared `Public X As Int = 5` in Main: does it print `5` or
   `0`? And the other way round?
3. **Missing `Process_Globals`.** Does B4J build a code module with no
   `Process_Globals`, and a call to its Sub print as usual?
4. **Unprefixed call.** Main calls `F` where only Module2 has a Sub `F`:
   does B4J build it (and print `F`'s output) or refuse it, with which
   message?
5. **Private members.** Main reads `Module2.P`, declared `Private`, and
   calls `Module2.G`, a `Private Sub`: refused, with which messages? Does
   `Dim` in `Process_Globals` make a variable public (`Module2.D` read from
   Main prints its value)?
6. **Same names in two modules.** Main and Module2 each have a Sub `F`
   printing its module: does `F` in Main call Main's and `Module2.F`
   Module2's? Does `module2.f` (other case) call it too?
7. **Names that collide with a module.** A local `Dim Module2 As Int`, a
   parameter named `Module2`, and a Sub named `Module2` in Main: does each
   build, and what does `Log(Module2)` print?
8. **Types across modules.** Module2 declares `Type Pt (X As Int)`: does
   Main's `Dim p As Pt` build, and `Dim p As Module2.Pt`? What does
   `Log(p)` print (its class name in the output)?

### Classes

9. **Class structure.** A class with no `Initialize`, one with no
   `Class_Globals`, and one with a `Process_Globals`: which does B4J build,
   and what does each print when used?
10. **`Class_Globals` rules.** In `Class_Globals`: a `Dim` with a Sub's name;
    a statement such as `Log("cg")`; a `For` over a variable declared with
    `Dim`: which build, and when does the `Log` print relative to the
    `Initialize` body?
11. **Properties.** With `getText`/`setText`, does `c.Text = "a"` then
    `Log(c.Text)` print `a`? With only `getText`, is `c.Text = "a"`
    refused? With `GetText` (capital `G`), is `c.Text` refused? Is a
    property named like a `Class_Globals` variable refused?
12. **Before `Initialize`.** On `Dim p As Person`, without `Initialize`:
    what do `p.IsInitialized`, `Log(p.Name)` (a public `Class_Globals`
    variable declared `= "x"`) and a call `p.Hello` (which logs) print, or
    which error kind is raised?
13. **`Initialize` twice.** `Class_Globals` has `Public N As Int = 1`; the
    program sets `p.N = 5`, calls `p.Initialize` again, and logs `p.N`: `1`
    (Class_Globals ran again) or `5`?
14. **`Me` in code modules.** Does B4J build `Log(Me)` in Main's `AppStart`
    and in a Sub of Module2, and what do they print? Does
    `CallSub(Me, "F")` in Module2 call Module2's `F`?
15. **Printing an instance.** A class with three public and one private
    `Class_Globals` variables: what do `Log(p)`, `Log("" & p)` and
    `Log(GetType(p))` print, before and after `Initialize` (field names,
    case, order, line breaks)?
16. **Identity and `Me`.** `p2 = p`, then `Log(p2 = p)`, `Log(p = q)` for
    another instance, `Log(p = Null)` on an uninitialised `p`; inside the
    class, `Log(Me = Other)` where `Other` is the instance passed in, and
    `Me.N` reading a `Class_Globals` variable: which build, what do they
    print?
17. **Instances in other modules.** Module2's `Process_Globals` declares
    `Public P As Person`; Main initialises `Module2.P` and calls
    `Module2.P.Hello`: does it build and print? Does `Log(Module2.P.N)` read
    the instance's variable?
18. **A class's events.** A class creates a `Timer` in `Initialize` with
    its own `Timer1_Tick` Sub, and Main also has a `Timer1_Tick`: which Sub
    runs (each logs its module)?

### Events

19. **Timer without a Tick Sub.** A program enables a `Timer` with no
    matching Sub, waits for two intervals in the message loop (a second
    timer stops it), then logs `end`: does B4J refuse it, print `end`, or
    end with an error (which kind, which exit code)?
20. **Event Sub signature.** `Sub Timer1_Tick (X As Int)`: refused, or an
    error at the first tick (which kind)?
21. **`#Event` lines.** A class with `#Event: Done (V As Int)` that raises
    `Done` with `CallSub2`: does the output differ from the same class
    without the line? Is `#Event` with no matching Sub refused?
22. **Sender.** In a `Timer1_Tick`, `Log(Sender = Timer1)`; then after a
    `CallSub(Me, "G")`, `Log(Sender = Null)`; in a Sub called directly from
    the tick, `Log(Sender = Timer1)`; and in `AppStart`, `Log(Sender = Null)`:
    what does each print, and which does B4J refuse?
23. **Sender after Sleep.** In a tick Sub, `Sleep(0)`, then
    `Log(Sender = Null)`: refused, `true`, or `false`?
24. **Message loop order.** `CallSubDelayed(Me, "A")`, then
    `StopMessageLoop`, then `CallSubDelayed(Me, "B")`, then
    `StartMessageLoop` and `Log("after")`: is the output `A`, `after` (B
    never runs)? And with `StopMessageLoop` called before any message is
    posted, does `StartMessageLoop` return at once?
25. **Errors in events.** A `CallSubDelayed` Sub raises an error not
    caught; Main has `Application_Error` that logs `handled` and returns
    `False`: does the program print `handled` and continue (a later
    `CallSubDelayed` logs `next`)? Without `Application_Error`, which exit
    code and which stdout?

### Resumable subs

26. **Return types.** A resumable sub declared `As Int`: refused, with
    which message? One declared `As ResumableSub` that returns `3` without
    pausing (its `Sleep` is behind an `If False`): does
    `Wait For (F) Complete (R As Int)` print `3`? What do `rs.Completed`
    and a second `Wait For (rs)` on it print or raise?
27. **Errors after a pause.** A Sub called before `StartMessageLoop` does
    `Sleep(0)`, then raises an error; `AppStart` has a `Try` around
    `StartMessageLoop` that logs `caught`: is `caught` printed, or does the
    program end with an error (which exit code)?
28. **Try around Sleep.** `Try`, `Sleep(0)`, an error, `Catch`,
    `Log("caught")`, `End Try`: refused, or does it print `caught`?
29. **Wait For parameters.** `Wait For E (V As Int)` resumed by
    `CallSub2(Me, "E", "5")` and by `CallSub2(Me, "E", 5.5)`: what is `V`,
    or which error kind? Does `CallSub2(Me, "e", 5)` (another case) resume
    it, and is the Sub `E` (if there is one) not called?

### CallSub and SubExists

30. **Module names as components.** `CallSub(Module2, "F")`,
    `SubExists(Module2, "F")`, `SubExists(Module2, "Missing")`,
    `CallSubDelayed(Module2, "F")` and `Log(Module2)` from Main: which
    build, and what do they print?
31. **Missing and private Subs.** `CallSub(p, "Missing")` on an initialised
    instance inside `Try` (logs `caught` and whether the message contains
    `was not found`); `CallSub(p, "Missing")` on an uninitialised one (logs
    the result `= Null`); `CallSub(p, "PrivateSub")`; and
    `CallSub(Module2, "Missing")`: what does each print?
32. **Errors in a called Sub.** `Try`, `CallSub(Me, "Boom")` where `Boom`
    divides by zero with `Mod`, `Catch`, `Log("caught")`, then
    `Log("after")`, in `AppStart`: are both lines printed, or does the
    program end with exit code 1? The same from a `CallSubDelayed` Sub,
    inside the loop.
33. **CallSubDelayed on an uninitialised instance.** `Try`,
    `CallSubDelayed(p, "Hello")` with `p` not initialised, `Catch`,
    `Log(LastException.Message.Contains("NullPointerException"))`: what is
    printed?

## jCore references

All at commit `e26e645a611dd24c2034fbbeaf11ba26b70dee8b`, under
`jCore/src/anywheresoftware/b4a/`:

| Name | Link |
| --- | --- |
| `B4AClass.ImplB4AClass` (`IsInitialized`, `toString`) | [B4AClass.java#L28-L44](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/B4AClass.java#L28-L44) |
| `BA.subExists`, `BA.raiseEvent2` | [BA.java#L99-L153](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L99-L153) |
| `BA.checkAndRunWaitForEvent` | [BA.java#L154-L169](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L154-L169) |
| `BA.handleUncaughtException` (`Application_Error`) | [BA.java#L171-L198](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L171-L198) |
| `BA.loadHtSubs` | [BA.java#L336-L344](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L336-L344) |
| `BA.getSender` | [BA.java#L394-L396](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L394-L396) |
| `BA.TypeToString` | [BA.java#L464-L503](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L464-L503) |
| `BA.ResumableSub`, `BA.WaitForEvent` | [BA.java#L604-L635](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/BA.java#L604-L635) |
| `StandardBA` (start, post, stop) | [StandardBA.java#L30-L54](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/StandardBA.java#L30-L54) |
| `SimpleMessageLoop` | [keywords/SimpleMessageLoop.java#L25-L49](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/SimpleMessageLoop.java#L25-L49) |
| `Common.StartMessageLoop`, `StopMessageLoop`, `Sender` | [keywords/Common.java#L178-L207](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L178-L207) |
| `Common.GetType` | [keywords/Common.java#L400-L405](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L400-L405) |
| `Common.CallSub*`, `CallSub4` | [keywords/Common.java#L457-L528](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L457-L528) |
| `Common.CallSubDelayed*`, `getComponentBA` | [keywords/Common.java#L529-L588](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L529-L588) |
| `Common.SubExists` | [keywords/Common.java#L590-L601](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L590-L601) |
| `Common.Initialized`, `NotInitialized` | [keywords/Common.java#L606-L655](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L606-L655) |
| `Common.Me` | [keywords/Common.java#L1073-L1080](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L1073-L1080) |
| `Common.Sleep` | [keywords/Common.java#L1081-L1122](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L1081-L1122) |
| `Common.WaitFor` | [keywords/Common.java#L1125-L1164](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L1125-L1164) |
| `Common.ReturnFromResumableSub`, `ResumableSubWrapper` | [keywords/Common.java#L1165-L1201](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/keywords/Common.java#L1165-L1201) |
| `Timer` | [objects/Timer.java#L33-L152](https://github.com/AnywhereSoftware/B4J/blob/e26e645a611dd24c2034fbbeaf11ba26b70dee8b/jCore/src/anywheresoftware/b4a/objects/Timer.java#L33-L152) |
