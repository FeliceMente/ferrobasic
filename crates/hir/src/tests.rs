use crate::hir::*;
use crate::{Module, Ty, check};

fn module(text: &str) -> (Module, Vec<String>) {
    let parse = ferro_syntax::parse(text);
    assert!(
        parse.errors().is_empty(),
        "syntax errors: {:?}",
        parse.errors()
    );
    let (module, diags) = check(&parse.syntax());
    (module, diags.into_iter().map(|d| d.message).collect())
}

/// A program whose `AppStart` body is `body`.
fn program(body: &str) -> String {
    format!("Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n{body}\nEnd Sub\n")
}

fn ok(text: &str) -> Module {
    let (module, diags) = module(text);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    module
}

fn errors(text: &str) -> Vec<String> {
    let (_, diags) = module(text);
    assert!(!diags.is_empty(), "expected diagnostics");
    diags
}

fn has(diags: &[String], fragment: &str) -> bool {
    diags.iter().any(|d| d.contains(fragment))
}

fn app_start(module: &Module) -> &Sub {
    module.sub(module.app_start.unwrap())
}

/// The value of the last assignment or Dim in `AppStart`.
fn last_value(module: &Module) -> Expr {
    app_start(module)
        .body
        .iter()
        .rev()
        .find_map(|s| match &s.kind {
            StmtKind::Assign { value, .. } => Some(value.clone()),
            StmtKind::Dim {
                init: DimInit::Value(value),
                ..
            } => Some(value.clone()),
            _ => None,
        })
        .expect("no assignment")
}

/// A compact rendering of an expression: `conv(Int->Double, 1)` etc.
fn show(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Literal(Literal::Int(i)) => i.to_string(),
        ExprKind::Literal(Literal::Long(i)) => format!("{i}L"),
        ExprKind::Literal(Literal::Double(d)) => format!("{d:?}"),
        ExprKind::Literal(Literal::String(s)) => format!("{s:?}"),
        ExprKind::Literal(lit) => format!("{lit:?}"),
        ExprKind::Var(_) => "var".into(),
        ExprKind::Convert(inner) => format!("conv({}->{}, {})", inner.ty, e.ty, show(inner)),
        ExprKind::Neg(inner) => format!("-{}", show(inner)),
        ExprKind::Arith(op, l, r) => format!("{op:?}:{}({}, {})", e.ty, show(l), show(r)),
        ExprKind::Concat(l, r) => format!("concat({}, {})", show(l), show(r)),
        ExprKind::Compare(op, kind, l, r) => format!("{op:?}:{kind:?}({}, {})", show(l), show(r)),
        ExprKind::Logic(op, l, r) => format!("{op:?}({}, {})", show(l), show(r)),
        ExprKind::Builtin { builtin, .. } => format!("{builtin:?}"),
        other => format!("{other:?}"),
    }
}

#[test]
fn sample_program_resolves() {
    let module = ok(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/level0/prices_map.bas"
    ))
    .unwrap());
    assert_eq!(module.globals.len(), 1);
    assert_eq!(module.globals[0].ty, Ty::Map);
    assert!(module.process_globals.is_some());
    // `Total = Total + Prices.Get(K)`: the Object from Get becomes a Double.
    let sum = app_start(&module)
        .body
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::ForEach { body, element, .. } => {
                assert_eq!(*element, Ty::Object);
                match &body[0].kind {
                    StmtKind::Assign { value, .. } => Some(value.clone()),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(show(&sum), "Add:Double(var, conv(Object->Double, MapGet))");
}

#[test]
fn implicit_conversions_are_explicit() {
    assert_eq!(
        show(&last_value(&ok(&program("Dim d As Double = 1")))),
        "conv(Int->Double, 1)"
    );
    assert_eq!(
        show(&last_value(&ok(&program("Dim s As String = \"n = \" & 5")))),
        "concat(\"n = \", conv(Int->String, 5))"
    );
    assert_eq!(
        show(&last_value(&ok(&program("Dim i As Int = \"12\"")))),
        "conv(String->Int, \"12\")"
    );
}

#[test]
fn any_value_converts_to_string() {
    ok(&program(
        "Try
  Log(1)
Catch
  Log(LastException)
  Log(\"error: \" & LastException)
End Try",
    ));
}

#[test]
fn arithmetic_types() {
    let m = ok(&program(
        "Dim a As Int = 55\nDim b As Int = 10\nDim d As Double = a / b",
    ));
    assert_eq!(
        show(&last_value(&m)),
        "Div:Double(conv(Int->Double, var), conv(Int->Double, var))"
    );
    let m = ok(&program("Dim a As Int = 1\nDim l As Long = a * 2"));
    assert_eq!(show(&last_value(&m)), "conv(Int->Long, Mul:Int(var, 2))");
    let m = ok(&program("Dim x As Long = 3000000000"));
    assert_eq!(show(&last_value(&m)), "3000000000L");
    let m = ok(&program("Dim h As Int = 0xFF000000"));
    assert_eq!(show(&last_value(&m)), "-16777216");
}

#[test]
fn comparisons() {
    let m = ok(&program("Dim b As Boolean = 1 < 2.5"));
    assert_eq!(
        show(&last_value(&m)),
        "Lt:Numeric(conv(Int->Double, 1), 2.5)"
    );
    let m = ok(&program("Dim b As Boolean = \"a\" = \"b\""));
    assert_eq!(show(&last_value(&m)), "Eq:String(\"a\", \"b\")");
    let m = ok(&program("Dim L As List\nDim b As Boolean = L = Null"));
    assert!(show(&last_value(&m)).starts_with("Eq:Object("));
}

#[test]
fn comparison_chains_are_type_errors() {
    // `(1 < 2) < 3` compares a Boolean with a number.
    let d = errors(&program("Dim b As Boolean = 1 < 2 < 3"));
    assert!(has(&d, "cannot compare Boolean with Int"), "{d:?}");
    // Assigning one comparison is fine.
    ok(&program("Dim b As Boolean\nDim n As Int = 1\nb = n = 1"));
}

#[test]
fn locals_are_hoisted_to_the_whole_sub() {
    let m = ok(&program(
        "For i = 1 To 3\n  If i = 2 Then\n    Dim inner As String = \"x\"\n  End If\nNext\nLog(inner)",
    ));
    let names: Vec<&str> = app_start(&m)
        .locals
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(names, ["Args", "inner", "i"]);
    assert_eq!(app_start(&m).locals[2].ty, Ty::Int);
}

#[test]
fn declaration_forms() {
    let m = ok(&program(
        "Dim a = 1, b = 2 As Int\nPrivate i, j As Long\nDim s\nDim m(3, 4) As Double\nDim days() As String = Array As String(\"x\")",
    ));
    let types: Vec<String> = app_start(&m)
        .locals
        .iter()
        .map(|v| v.ty.to_string())
        .collect();
    assert_eq!(
        types,
        [
            "String()",
            "Int",
            "Int",
            "Long",
            "Long",
            "String",
            "Double(,)",
            "String()"
        ]
    );
}

#[test]
fn conflicting_declarations() {
    let d = errors(&program("Dim x As Int\nDim x As String"));
    assert!(has(&d, "already declared as Int"), "{d:?}");
    ok(&program("Dim x As Int\nDim x As Int"));
}

#[test]
fn undeclared_names() {
    let d = errors(&program("y = 5"));
    assert!(has(&d, "`y` is not declared"), "{d:?}");
    let d = errors(&program("Log(z)"));
    assert!(has(&d, "`z` is not declared"), "{d:?}");
}

#[test]
fn later_level_features_have_precise_messages() {
    let d = errors(&program("Sleep(100)"));
    assert!(has(&d, "Level 2"), "{d:?}");
    let d = errors(&program("Log(DateTime.Now)"));
    assert!(has(&d, "DateTime"), "{d:?}");
    let d = errors("Sub Class_Globals\nEnd Sub\nSub AppStart (Args() As String)\nEnd Sub\n");
    assert!(has(&d, "Level 1"), "{d:?}");
    let d = errors(&program("Dim b As Button"));
    assert!(has(&d, "unknown type `Button`"), "{d:?}");
    let d = errors("Sub AppStart (Form1 As Form, Args() As String)\nEnd Sub\n");
    assert!(has(&d, "UI apps"), "{d:?}");
    let d = errors("#If JAVA\n#End If\nSub AppStart (Args() As String)\nEnd Sub\n");
    assert!(has(&d, "#If"), "{d:?}");
}

#[test]
fn missing_entry_point() {
    let d = errors("Sub Process_Globals\nEnd Sub\n");
    assert!(has(&d, "missing `Sub AppStart"), "{d:?}");
}

#[test]
fn members_and_builtins() {
    ok(&program(
        "Dim L As List\nL.Initialize\nL.Add(1)\nLog(L.Size & \" \" & L.Get(0))\n\
         Dim s As String = \"abc\"\nLog(s.ToUpperCase.SubString2(0, 2))\n\
         Log(Bit.And(6, 3))\nDim o As Object = 5\nDim i As Int = o.As(Int)\n\
         Dim sb As StringBuilder\nsb.Initialize\nsb.Append(\"a\").Append(\"b\")\n\
         Log(Abs(-2) & cPI & CRLF)\nLog(IIf(i > 1, \"big\", \"small\"))",
    ));
    let d = errors(&program("Dim L As List\nL.Put(1, 2)"));
    assert!(has(&d, "List has no member `Put`"), "{d:?}");
    let d = errors(&program("Dim o As Object\nLog(o.Size)"));
    assert!(has(&d, "on an Object"), "{d:?}");
    let d = errors(&program("Log(1, 2)"));
    assert!(has(&d, "expects 1 argument(s) but 2"), "{d:?}");
    let d = errors(&program("Dim m As Map\nDim x As Object = m.Keys"));
    assert!(has(&d, "only be used in `For Each`"), "{d:?}");
}

#[test]
fn subs_calls_and_returns() {
    ok(
        "Sub AppStart (Args() As String)\n  Log(Square(12))\n  Hello\nEnd Sub\n\
        Sub Square (x As Int) As Int\n  Return x * x\nEnd Sub\nSub Hello\n  Log(\"hi\")\nEnd Sub\n",
    );
    let d = errors("Sub AppStart (Args() As String)\nEnd Sub\nSub F\n  Return 1\nEnd Sub\n");
    assert!(has(&d, "no return type"), "{d:?}");
    let d =
        errors("Sub AppStart (Args() As String)\n  Dim x As Int = F\nEnd Sub\nSub F\nEnd Sub\n");
    assert!(has(&d, "does not return a value"), "{d:?}");
    let d = errors(
        "Sub AppStart (Args() As String)\nEnd Sub\nSub AppStart (Args() As String)\nEnd Sub\n",
    );
    assert!(has(&d, "defined twice"), "{d:?}");
}

#[test]
fn exit_and_continue_placement() {
    ok(&program(
        "Select 1\n  Case 1\n    Exit\nEnd Select\nDo While True\n  Continue\nLoop",
    ));
    let d = errors(&program("Exit"));
    assert!(has(&d, "inside a loop or a `Select`"), "{d:?}");
    let d = errors(&program("Select 1\n  Case 1\n    Continue\nEnd Select"));
    assert!(has(&d, "`Continue` must be inside a loop"), "{d:?}");
}

#[test]
fn records_and_arrays() {
    ok(
        "Sub Process_Globals\n  Type Person (Name As String, Scores(3) As Int)\nEnd Sub\n\
        Sub AppStart (Args() As String)\n  Dim p As Person\n  p.Initialize\n  p.Name = \"Ann\"\n  p.Scores(0) = 10\n\
          Dim a(2, 2) As Int\n  a(1, 1) = 5\n  Log(p.Name & a(1, 1) & a.Length)\nEnd Sub\n",
    );
    let d = errors(&program("Dim a(3) As Int\na(1, 2) = 0"));
    assert!(has(&d, "1 dimension(s) but 2"), "{d:?}");
}

#[test]
fn smart_string_formatters() {
    ok(&program(
        "Dim x As Double = 2.5\nLog($\"${x} $1.2{x} $XML{\"<\"}\"$)",
    ));
    let d = errors(&program("Log($\"$date{1}\"$)"));
    assert!(has(&d, "DateTime"), "{d:?}");
    let d = errors(&program("Log($\"$foo{1}\"$)"));
    assert!(has(&d, "unknown smart string formatter"), "{d:?}");
}

#[test]
fn for_each_requires_an_iterable() {
    let d = errors(&program("Dim n As Int\nFor Each x As Int In n\nNext"));
    assert!(has(&d, "cannot iterate over Int"), "{d:?}");
    let d = errors(&program("Dim L As List\nFor Each q In L\nNext"));
    assert!(has(&d, "`q` is not declared"), "{d:?}");
}

#[test]
fn select_kinds() {
    let m = ok(&program(
        "Dim s As String = \"a\"\nSelect s\n  Case \"a\", \"b\"\n  Case Else\nEnd Select",
    ));
    let kind = app_start(&m).body.iter().find_map(|s| match &s.kind {
        StmtKind::Select {
            kind,
            cases,
            else_block,
            ..
        } => {
            assert_eq!(cases[0].values.len(), 2);
            assert!(else_block.is_some());
            Some(*kind)
        }
        _ => None,
    });
    // Strings match with `equals`, like any non-numeric value.
    assert_eq!(kind, Some(CmpKind::Object));
}

#[test]
fn array_initializer_is_kept() {
    let m = ok(&program(
        "Dim days() As String = Array As String(\"Mon\", \"Tue\")",
    ));
    match &app_start(&m).body[0].kind {
        StmtKind::Dim {
            init: DimInit::Value(v),
            ..
        } => {
            assert!(matches!(v.kind, ExprKind::ArrayLiteral(ref items) if items.len() == 2));
        }
        other => panic!("{other:?}"),
    }
    let d = errors(&program("Dim a() As Int = missing"));
    assert!(has(&d, "`missing` is not declared"), "{d:?}");
    let d = errors(&program("Dim a(3) As Int = Array As Int(1)"));
    assert!(has(&d, "cannot also have an initial value"), "{d:?}");
}

#[test]
fn too_many_dimensions_is_a_diagnostic() {
    let dims = vec!["1"; 256].join(", ");
    let d = errors(&program(&format!("Dim a({dims}) As Int\nDim s As Map = a")));
    assert!(has(&d, "at most 255 dimensions"), "{d:?}");
    let d = errors(&format!(
        "Sub Process_Globals\n  Type R (F({dims}) As Int)\nEnd Sub\nSub AppStart (Args() As String)\nEnd Sub\n"
    ));
    assert!(has(&d, "at most 255 dimensions"), "{d:?}");
}

#[test]
fn record_field_bounds_see_globals() {
    ok(
        "Sub Process_Globals\n  Private n As Int = 3\n  Type Row (Items(n) As Int)\nEnd Sub\n\
        Sub AppStart (Args() As String)\nEnd Sub\n",
    );
}

#[test]
fn select_matches_like_switch_object_to_int() {
    // A String scrutinee compares with `equals`: Case 1 stays a boxed Int.
    let m = ok(&program(
        "Select \"1\"\n  Case 1\n    Log(\"one\")\nEnd Select",
    ));
    match &app_start(&m).body[0].kind {
        StmtKind::Select { kind, cases, .. } => {
            assert_eq!(*kind, CmpKind::Object);
            assert_eq!(show(&cases[0].values[0]), "conv(Int->Object, 1)");
        }
        other => panic!("{other:?}"),
    }
    // A Char scrutinee converts its cases to Char (docs: `Select Txt.CharAt(i)`).
    let m = ok(&program(
        "Dim t As String = \"AB\"\nSelect t.CharAt(0)\n  Case \"A\", \"B\"\nEnd Select",
    ));
    let kind = app_start(&m).body.iter().find_map(|s| match &s.kind {
        StmtKind::Select { kind, .. } => Some(*kind),
        _ => None,
    });
    assert_eq!(kind, Some(CmpKind::Char));
}

#[test]
fn inline_cast_to_array_type() {
    ok(&program("Dim o As Object\nLog(o.As(String()).Length)"));
}

/// A `For` in `Process_Globals` with an implicit counter used to vanish
/// silently; it is rejected until B4X's behaviour is known.
#[test]
fn loops_in_process_globals_need_a_declared_variable() {
    let globals = |body: &str| {
        format!(
            "Sub Process_Globals\n    Private Total As Int\n{body}\nEnd Sub\n\nSub AppStart (Args() As String)\n    Log(Total)\nEnd Sub\n"
        )
    };
    let diags = errors(&globals(
        "    For i = 1 To 3\n        Total = Total + i\n    Next",
    ));
    assert!(
        has(
            &diags,
            "the `For` variable `i` must be declared first in `Process_Globals`"
        ),
        "{diags:?}"
    );
    let diags = errors(&globals("    For i As Int = 1 To 3\n    Next"));
    assert!(
        has(&diags, "the `For` variable `i` must be declared first"),
        "{diags:?}"
    );
    let diags = errors(&globals(
        "    For Each v As Int In Array As Int(1, 2)\n    Next",
    ));
    assert!(
        has(&diags, "the `For Each` variable `v` must be declared first"),
        "{diags:?}"
    );
    // A declared variable works, as before.
    let module = ok(&globals(
        "    Dim i As Int\n    For i = 1 To 3\n        Total = Total + i\n    Next",
    ));
    let pg = module.sub(module.process_globals.unwrap());
    assert!(
        format!("{:?}", pg.body).contains("For"),
        "the loop must be kept"
    );
}

#[test]
fn assigning_to_a_constant_or_a_sub_says_so() {
    let diags = errors(&program("    CRLF = \"x\"\n    cPI = 3"));
    assert!(
        has(&diags, "cannot assign to `CRLF`: it is a built-in constant"),
        "{diags:?}"
    );
    assert!(
        has(&diags, "cannot assign to `cPI`: it is a built-in constant"),
        "{diags:?}"
    );
    let diags = errors(&format!("{}\nSub F\nEnd Sub\n", program("    F = 3")));
    assert!(
        has(&diags, "cannot assign to `F`: it is a Sub, not a variable"),
        "{diags:?}"
    );
    let diags = errors(&program("    nothing = 3"));
    assert!(has(&diags, "`nothing` is not declared"), "{diags:?}");
}
