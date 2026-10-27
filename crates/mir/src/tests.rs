//! Unit tests for the HIR → MIR desugarings. They assert on the shape of
//! the blocks, not only on validity.

use crate::*;

fn lower_text(text: &str) -> Program {
    let parse = ferro_syntax::parse(text);
    assert!(
        parse.errors().is_empty(),
        "syntax errors: {:?}",
        parse.errors()
    );
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let program = lower(&module);
    if let Err(e) = validate(&program) {
        panic!("{e}\n{}", dump(&program));
    }
    program
}

/// A program whose `AppStart` body is `body`, after `extra` Subs.
fn with_subs(extra: &str, body: &str) -> Program {
    lower_text(&format!(
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n{body}\nEnd Sub\n\n{extra}\n"
    ))
}

fn program(body: &str) -> Program {
    with_subs("", body)
}

/// `Operand` has no `PartialEq` (it is not part of the contract).
fn same(a: &Operand, b: &Operand) -> bool {
    match (a, b) {
        (Operand::Local(x), Operand::Local(y)) => x == y,
        (Operand::Const(x), Operand::Const(y)) => x == y,
        _ => false,
    }
}

fn int(i: i32) -> Operand {
    Operand::Const(Constant::Int(i))
}

fn text(s: &str) -> Operand {
    Operand::Const(Constant::String(s.into()))
}

fn app(p: &Program) -> &Function {
    p.function(p.entry)
}

fn func<'p>(p: &'p Program, name: &str) -> &'p Function {
    p.functions.iter().find(|f| f.name == name).unwrap()
}

fn func_id(p: &Program, name: &str) -> FuncId {
    FuncId(p.functions.iter().position(|f| f.name == name).unwrap() as u32)
}

fn rvalues(stmt: &Statement) -> Option<&Rvalue> {
    match &stmt.kind {
        StatementKind::Assign { value, .. } | StatementKind::Eval(value) => Some(value),
        _ => None,
    }
}

fn all_rvalues(f: &Function) -> impl Iterator<Item = &Rvalue> {
    f.blocks
        .iter()
        .flat_map(|b| b.stmts.iter().filter_map(rvalues))
}

/// The blocks that contain a statement matching `pred`.
fn blocks_where(f: &Function, pred: impl Fn(&Statement) -> bool) -> Vec<BlockId> {
    f.blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.stmts.iter().any(&pred))
        .map(|(i, _)| BlockId(i as u32))
        .collect()
}

/// The single block that contains a statement matching `pred`.
fn block_where(f: &Function, pred: impl Fn(&Statement) -> bool) -> BlockId {
    let found = blocks_where(f, pred);
    assert_eq!(found.len(), 1, "expected one block, found {found:?}");
    found[0]
}

/// Whether the statement logs the string constant `text`.
fn logs(text: &str) -> impl Fn(&Statement) -> bool + '_ {
    move |s| {
        matches!(&s.kind, StatementKind::Eval(Rvalue::Builtin { builtin: Builtin::Log, args, .. })
            if args.first().is_some_and(|a| same(a, &self::text(text))))
    }
}

/// Whether the statement calls `func` with the Int constant `arg` first.
fn calls(func: FuncId, arg: i32) -> impl Fn(&Statement) -> bool {
    move |s| {
        matches!(rvalues(s), Some(Rvalue::Call { func: f, args })
            if *f == func && args.first().is_some_and(|a| same(a, &int(arg))))
    }
}

fn is_arith_add(s: &Statement) -> bool {
    matches!(
        &s.kind,
        StatementKind::Assign {
            value: Rvalue::Arith {
                op: ArithOp::Add,
                ..
            },
            ..
        }
    )
}

fn compares(f: &Function, op: CmpOp) -> usize {
    all_rvalues(f)
        .filter(|r| matches!(r, Rvalue::Compare { op: o, .. } if *o == op))
        .count()
}

fn branch(f: &Function, b: BlockId) -> (BlockId, BlockId) {
    match &f.block(b).terminator {
        Terminator::Branch {
            then, otherwise, ..
        } => (*then, *otherwise),
        t => panic!("bb{} ends with {t:?}", b.0),
    }
}

fn goto(f: &Function, b: BlockId) -> BlockId {
    match &f.block(b).terminator {
        Terminator::Goto(t) => *t,
        t => panic!("bb{} ends with {t:?}", b.0),
    }
}

fn local_named(f: &Function, name: &str) -> LocalId {
    LocalId(
        f.locals
            .iter()
            .position(|l| l.name.as_deref() == Some(name))
            .unwrap() as u32,
    )
}

#[test]
fn layout_with_an_empty_process_globals() {
    let p = lower_text(
        "Sub Process_Globals\nEnd Sub\nSub AppStart (Args() As String)\nLog(\"hi\")\nEnd Sub\n",
    );
    assert_eq!(p.function(p.init).name, "Process_Globals");
    assert!(p.function(p.init).blocks[0].stmts.is_empty());
    assert_eq!(app(&p).name, "AppStart");
    assert_eq!(app(&p).params, 1);
}

#[test]
fn for_with_negative_constant_step() {
    let p = program("For i = 10 To 1 Step -2\nLog(i)\nNext");
    let f = app(&p);
    // Only the `>=` test, no run-time test of the step's sign.
    assert_eq!(compares(f, CmpOp::Ge), 1);
    assert_eq!(compares(f, CmpOp::Le), 0);
    assert_eq!(compares(f, CmpOp::Gt), 0);
    assert_eq!(compares(f, CmpOp::Lt), 0);
    let i = local_named(f, "i");
    let incr = block_where(f, is_arith_add);
    assert!(matches!(
        &f.block(incr).stmts[0].kind,
        StatementKind::Assign { dest, value: Rvalue::Arith { ty: Ty::Int, left: Operand::Local(l), .. } }
            if *dest == i && *l == i
    ));
}

#[test]
fn for_with_default_step_adds_one() {
    let p = program("For i = 1 To 3\nLog(i)\nNext");
    let f = app(&p);
    assert_eq!(compares(f, CmpOp::Le), 1);
    assert_eq!(compares(f, CmpOp::Ge), 0);
    let incr = block_where(f, is_arith_add);
    assert!(matches!(
        &f.block(incr).stmts[0].kind,
        StatementKind::Assign {
            value: Rvalue::Arith {
                right: Operand::Const(Constant::Int(1)),
                ..
            },
            ..
        }
    ));
}

#[test]
fn for_with_variable_step_tests_the_sign() {
    let p = with_subs(
        "Sub Limit As Int\nReturn 1\nEnd Sub",
        "Dim s As Int = -1\nFor i = 3 To Limit Step s\ns = 5\nLog(i)\nNext",
    );
    let f = app(&p);
    assert_eq!(compares(f, CmpOp::Gt), 1);
    assert_eq!(compares(f, CmpOp::Lt), 1);
    assert_eq!(compares(f, CmpOp::Le), 1);
    assert_eq!(compares(f, CmpOp::Ge), 1);
    // The limit is evaluated once, before the loop.
    let limit = func_id(&p, "Limit");
    let call = blocks_where(
        f,
        |s| matches!(rvalues(s), Some(Rvalue::Call { func, .. }) if *func == limit),
    );
    assert_eq!(call, vec![BlockId(0)]);
    // The step is a copy of `s` taken before the loop, not `s` itself.
    let s = local_named(f, "s");
    let incr = block_where(f, is_arith_add);
    let StatementKind::Assign {
        value: Rvalue::Arith {
            right: Operand::Local(step),
            ..
        },
        ..
    } = &f.block(incr).stmts[0].kind
    else {
        panic!("no increment");
    };
    assert_ne!(*step, s);
    assert!(f.local(*step).name.is_none());
}

#[test]
fn for_each_over_array_list_and_map_keys() {
    let p = program(
        "Dim a() As Int = Array As Int(1, 2)\nFor Each x As Int In a\nLog(x)\nNext\n\
         Dim l As List\nl.Initialize\nFor Each s As String In l\nLog(s)\nNext\n\
         Dim m As Map = CreateMap(\"k\": 1)\nFor Each k As String In m.Keys\nLog(k)\nNext",
    );
    let f = app(&p);
    let sizes = all_rvalues(f)
        .filter(|r| matches!(r, Rvalue::IterSize(_)))
        .count();
    let gets = blocks_where(f, |s| matches!(rvalues(s), Some(Rvalue::IterGet { .. })));
    assert_eq!(sizes, 3);
    assert_eq!(gets.len(), 3);
    // IterSize is read before each loop, never in the block that reads elements.
    for g in &gets {
        assert!(
            !f.block(*g)
                .stmts
                .iter()
                .any(|s| matches!(rvalues(s), Some(Rvalue::IterSize(_))))
        );
    }
    // Int elements of an Int array need no conversion; List and Map views
    // give Objects converted to String.
    let converts: Vec<_> = all_rvalues(f)
        .filter_map(|r| match r {
            Rvalue::Convert { from, to, .. } if *from == Ty::Object => Some(to.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(converts, vec![Ty::String, Ty::String]);
    let x = local_named(f, "x");
    let first = &f.block(gets[0]).stmts;
    assert!(matches!(
        &first[1].kind,
        StatementKind::Assign { dest, value: Rvalue::Use(Operand::Local(_)) } if *dest == x
    ));
    assert!(all_rvalues(f).any(|r| matches!(
        r,
        Rvalue::Builtin {
            builtin: Builtin::MapKeys,
            ..
        }
    )));
}

#[test]
fn for_each_continue_goes_to_the_index_increment() {
    let p = program(
        "Dim a() As Int = Array As Int(1, 2)\nFor Each x As Int In a\nIf x = 1 Then Continue\nLog(\"after\")\nNext",
    );
    let f = app(&p);
    let incr = block_where(f, is_arith_add);
    // The `Then` block of the If jumps straight to the increment.
    let compare = block_where(f, |s| {
        matches!(rvalues(s), Some(Rvalue::Compare { op: CmpOp::Eq, .. }))
    });
    let (then, _) = branch(f, compare);
    assert_eq!(goto(f, then), incr);
}

#[test]
fn select_with_case_else() {
    let p = program(
        "Dim x As Int = 2\nSelect x\nCase 1, 2\nLog(\"a\")\nCase 3\nLog(\"b\")\nCase Else\nLog(\"c\")\nEnd Select\nLog(\"end\")",
    );
    let f = app(&p);
    assert_eq!(compares(f, CmpOp::Eq), 3);
    let a = block_where(f, logs("a"));
    let b = block_where(f, logs("b"));
    let c = block_where(f, logs("c"));
    let end = block_where(f, logs("end"));
    // Test chain: 1 -> a | 2 -> a | 3 -> b | else.
    let tests = blocks_where(f, |s| {
        matches!(rvalues(s), Some(Rvalue::Compare { op: CmpOp::Eq, .. }))
    });
    assert_eq!(tests.len(), 3);
    let (t1, n1) = branch(f, tests[0]);
    assert_eq!(t1, a);
    assert_eq!(n1, tests[1]);
    let (t2, n2) = branch(f, tests[1]);
    assert_eq!(t2, a);
    assert_eq!(n2, tests[2]);
    let (t3, n3) = branch(f, tests[2]);
    assert_eq!(t3, b);
    assert_eq!(n3, c);
    // No fall-through: every body goes to the end.
    for body in [a, b, c] {
        assert_eq!(goto(f, body), end);
    }
    // The scrutinee is a snapshot of x, compared as a Double.
    for r in all_rvalues(f) {
        if let Rvalue::Compare { kind, .. } = r {
            assert_eq!(*kind, CmpKind::Numeric);
        }
    }
}

#[test]
fn exit_inside_select_inside_loop_leaves_the_select() {
    let p = program(
        "Dim count As Int = 0\nFor i = 1 To 5\nSelect i\nCase 3\nExit\nEnd Select\ncount = count + 1\nNext\nLog(\"done\")",
    );
    let f = app(&p);
    let count = local_named(f, "count");
    let tests = blocks_where(f, |s| {
        matches!(rvalues(s), Some(Rvalue::Compare { op: CmpOp::Eq, .. }))
    });
    let (case_body, _) = branch(f, tests[0]);
    let after_select = goto(f, case_body);
    assert!(
        f.block(after_select)
            .stmts
            .iter()
            .any(|s| matches!(&s.kind, StatementKind::Assign { dest, .. } if *dest == count))
    );
}

#[test]
fn exit_and_continue_in_loops() {
    let p = program(
        "Dim i As Int = 0\nDo While i < 10\ni = i + 1\nIf i = 2 Then Continue\nIf i = 5 Then Exit\nLog(\"body\")\nLoop\nLog(\"done\")",
    );
    let f = app(&p);
    let head = goto(f, BlockId(0));
    let done = block_where(f, logs("done"));
    let eqs = blocks_where(f, |s| {
        matches!(rvalues(s), Some(Rvalue::Compare { op: CmpOp::Eq, .. }))
    });
    let (cont, _) = branch(f, eqs[0]);
    assert_eq!(goto(f, cont), head);
    let (exit, _) = branch(f, eqs[1]);
    assert_eq!(goto(f, exit), done);
    // Do While tests before the body: the head branches to body or done.
    let (_, out) = branch(f, head);
    assert_eq!(out, done);
}

#[test]
fn continue_through_a_select_goes_to_the_loop() {
    let p = program("For i = 1 To 3\nSelect i\nCase 2\nContinue\nEnd Select\nLog(\"x\")\nNext");
    let f = app(&p);
    let incr = block_where(f, is_arith_add);
    let eq = block_where(f, |s| {
        matches!(rvalues(s), Some(Rvalue::Compare { op: CmpOp::Eq, .. }))
    });
    let (case_body, _) = branch(f, eq);
    assert_eq!(goto(f, case_body), incr);
}

#[test]
fn do_until_branches_out_when_true() {
    let p = program("Dim i As Int = 0\nDo Until i = 3\ni = i + 1\nLoop\nLog(\"done\")");
    let f = app(&p);
    let head = goto(f, BlockId(0));
    let done = block_where(f, logs("done"));
    let (then, _) = branch(f, head);
    assert_eq!(then, done);
}

#[test]
fn nested_try_handlers() {
    let p = program(
        "Try\nLog(\"a\")\nTry\nLog(\"b\")\nCatch\nLog(\"c\")\nEnd Try\nLog(\"d\")\n\
         Catch\nLog(\"e\")\nEnd Try\nLog(\"f\")",
    );
    let f = app(&p);
    let block = |t| block_where(f, logs(t));
    let handler = |t| f.block(block(t)).handler;
    let (outer, inner) = (block("e"), block("c"));
    assert_eq!(handler("a"), Some(outer));
    assert_eq!(handler("b"), Some(inner));
    assert_eq!(handler("c"), Some(outer));
    assert_eq!(handler("d"), Some(outer));
    assert_eq!(handler("e"), None);
    assert_eq!(handler("f"), None);
    assert_eq!(f.blocks[0].handler, None);
}

#[test]
fn try_covers_nested_blocks() {
    let p =
        program("Try\nFor i = 1 To 2\nIf i = 1 Then Log(\"x\")\nNext\nCatch\nLog(\"e\")\nEnd Try");
    let f = app(&p);
    let catch = block_where(f, logs("e"));
    assert_eq!(f.block(block_where(f, logs("x"))).handler, Some(catch));
    assert_eq!(f.block(block_where(f, is_arith_add)).handler, Some(catch));
}

const PREDICATE: &str = "Sub F (n As Int) As Boolean\nReturn n > 0\nEnd Sub";

#[test]
fn and_short_circuits() {
    let p = with_subs(PREDICATE, "If F(1) And F(2) Then Log(\"x\")");
    let f = app(&p);
    let fid = func_id(&p, "F");
    let left = block_where(f, calls(fid, 1));
    let right = block_where(f, calls(fid, 2));
    assert_ne!(left, right);
    let (then, otherwise) = branch(f, left);
    assert_eq!(then, right);
    // The short path assigns False and does not evaluate the right side.
    let short = &f.block(otherwise).stmts;
    assert!(matches!(
        &short[..],
        [Statement {
            kind: StatementKind::Assign {
                value: Rvalue::Use(Operand::Const(Constant::Bool(false))),
                ..
            },
            ..
        }]
    ));
    assert_eq!(goto(f, otherwise), goto(f, right));
}

#[test]
fn or_short_circuits() {
    let p = with_subs(PREDICATE, "Dim b As Boolean = F(1) Or F(2)\nLog(b)");
    let f = app(&p);
    let fid = func_id(&p, "F");
    let left = block_where(f, calls(fid, 1));
    let right = block_where(f, calls(fid, 2));
    let (then, otherwise) = branch(f, left);
    assert_eq!(otherwise, right);
    assert!(matches!(
        &f.block(then).stmts[..],
        [Statement {
            kind: StatementKind::Assign {
                value: Rvalue::Use(Operand::Const(Constant::Bool(true))),
                ..
            },
            ..
        }]
    ));
}

#[test]
fn iif_evaluates_only_the_chosen_value() {
    let p = with_subs(
        "Sub G (n As Int) As Int\nReturn n\nEnd Sub",
        "Dim c As Boolean = True\nDim r As Int = IIf(c, G(1), G(2))\nLog(r)",
    );
    let f = app(&p);
    let gid = func_id(&p, "G");
    let a = block_where(f, calls(gid, 1));
    let b = block_where(f, calls(gid, 2));
    let (then, otherwise) = branch(f, BlockId(0));
    assert_eq!((then, otherwise), (a, b));
    assert_eq!(goto(f, a), goto(f, b));
}

#[test]
fn smart_strings() {
    let p = program("Dim x As Int = 5\nDim y As Double = 1.5\nLog($\"a${x}b$1.2{y}\"$)");
    let f = app(&p);
    let format = all_rvalues(f)
        .find_map(|r| match r {
            Rvalue::Format { format, .. } => Some(format.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(format, "1.2");
    let parts = all_rvalues(f)
        .find_map(|r| match r {
            Rvalue::Concat(parts) => Some(parts.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(parts.len(), 4);
    assert!(same(&parts[0], &text("a")));
    assert!(same(&parts[2], &text("b")));
    // `${x}` is x converted to String, not a Format.
    let formats = all_rvalues(f)
        .filter(|r| matches!(r, Rvalue::Format { .. }))
        .count();
    assert_eq!(formats, 1);
    assert!(all_rvalues(f).any(|r| matches!(
        r,
        Rvalue::Convert {
            from: Ty::Int,
            to: Ty::String,
            ..
        }
    )));
}

#[test]
fn concatenation_is_flattened() {
    let p = program("Dim s As String = \"a\" & \"b\" & \"c\"\nLog(s)");
    let f = app(&p);
    let parts = all_rvalues(f)
        .find_map(|r| match r {
            Rvalue::Concat(parts) => Some(parts.len()),
            _ => None,
        })
        .unwrap();
    assert_eq!(parts, 3);
}

#[test]
fn dim_resets_in_a_loop_and_arrays() {
    let p = program(
        "For i = 1 To 2\nDim l As List\nl.Initialize\nDim a(3) As Int\nDim b() As String\nNext",
    );
    let f = app(&p);
    let l = local_named(f, "l");
    let a = local_named(f, "a");
    let b = local_named(f, "b");
    let body = block_where(
        f,
        |s| matches!(&s.kind, StatementKind::Assign { dest, value: Rvalue::Default(Ty::List) } if *dest == l),
    );
    assert_ne!(body, BlockId(0), "the Dim runs inside the loop");
    let stmts = &f.block(body).stmts;
    assert!(stmts.iter().any(|s| matches!(&s.kind,
        StatementKind::Assign { dest, value: Rvalue::NewArray { elem: Ty::Int, dims } }
            if *dest == a && dims.len() == 1 && same(&dims[0], &int(3)))));
    assert!(stmts.iter().any(|s| matches!(&s.kind,
        StatementKind::Assign { dest, value: Rvalue::NewArray { elem: Ty::String, dims } }
            if *dest == b && dims.len() == 1 && same(&dims[0], &int(0)))));
}

#[test]
fn globals_are_read_and_written() {
    let p = lower_text(
        "Sub Process_Globals\nPrivate g As Int = 1\nEnd Sub\n\n\
         Sub AppStart (Args() As String)\ng = g + 1\nLog(g)\nEnd Sub\n",
    );
    assert_eq!(p.globals.len(), 1);
    assert_eq!(p.globals[0].ty, Ty::Int);
    let init = p.function(p.init);
    assert!(matches!(
        &init.blocks[0].stmts[0].kind,
        StatementKind::SetGlobal {
            global: GlobalId(0),
            value: Operand::Const(Constant::Int(1))
        }
    ));
    let f = app(&p);
    let stmts = &f.blocks[0].stmts;
    assert!(matches!(
        &stmts[0].kind,
        StatementKind::Assign {
            value: Rvalue::Global(GlobalId(0)),
            ..
        }
    ));
    assert!(stmts.iter().any(|s| matches!(
        &s.kind,
        StatementKind::SetGlobal {
            global: GlobalId(0),
            ..
        }
    )));
}

#[test]
fn record_init_functions() {
    let p = lower_text(
        "Sub Process_Globals\nType Point (X As Int, Name As String, Items(3) As Double)\nEnd Sub\n\n\
         Sub AppStart (Args() As String)\nDim p As Point\np.Initialize\np.X = 2\nLog(p.X)\nEnd Sub\n",
    );
    assert_eq!(p.records.len(), 1);
    let record = &p.records[0];
    assert_eq!(record.name, "Point");
    let init = p.function(record.init);
    assert_eq!(init.name, "Point.Initialize");
    assert_eq!(init.params, 1);
    let sets: Vec<_> = init.blocks[0]
        .stmts
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::SetField { field, value, .. } => Some((*field, value.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(sets.len(), 3);
    let value_of = |op: &Operand| match op {
        Operand::Local(l) => init.blocks[0].stmts.iter().find_map(|s| match &s.kind {
            StatementKind::Assign { dest, value } if dest == l => Some(value.clone()),
            _ => None,
        }),
        _ => None,
    };
    assert!(matches!(
        value_of(&sets[0].1),
        Some(Rvalue::Default(Ty::Int))
    ));
    assert!(matches!(
        value_of(&sets[1].1),
        Some(Rvalue::Default(Ty::String))
    ));
    assert!(matches!(
        value_of(&sets[2].1),
        Some(Rvalue::NewArray { elem: Ty::Double, dims }) if dims.len() == 1 && same(&dims[0], &int(3))
    ));
    // `p.X = 2` and `p.X` use the field index.
    let f = app(&p);
    assert!(f.blocks[0].stmts.iter().any(|s| matches!(
        &s.kind,
        StatementKind::SetField {
            field: 0,
            value: Operand::Const(Constant::Int(2)),
            ..
        }
    )));
    assert!(all_rvalues(f).any(|r| matches!(r, Rvalue::Field { field: 0, .. })));
}

#[test]
fn return_values_and_fall_through() {
    let p = with_subs(
        "Sub Twice (n As Int) As Int\nIf n > 0 Then\nReturn n * 2\nEnd If\nEnd Sub",
        "Log(Twice(2))",
    );
    let f = func(&p, "Twice");
    assert_eq!(f.ret, Ty::Int);
    assert!(
        f.blocks
            .iter()
            .any(|b| matches!(b.terminator, Terminator::Return(Some(Operand::Local(_)))))
    );
    // When the If is false, control falls through to `Return(None)`.
    let test = block_where(f, |s| {
        matches!(rvalues(s), Some(Rvalue::Compare { op: CmpOp::Gt, .. }))
    });
    let (_, mut b) = branch(f, test);
    while let Terminator::Goto(next) = f.block(b).terminator {
        b = next;
    }
    assert!(matches!(f.block(b).terminator, Terminator::Return(None)));
}

#[test]
fn statements_carry_their_source_range() {
    let text = "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\nFor i = 1 To 2\nLog(\"in loop\")\nNext\nEnd Sub\n";
    let p = lower_text(text);
    let f = app(&p);
    let b = block_where(f, logs("in loop"));
    let range = f.block(b).stmts[0].range;
    assert_eq!(&text[range], "Log(\"in loop\")");
}

#[test]
fn dump_is_readable() {
    let p = program("For i = 1 To 2\nLog(i)\nNext");
    let text = dump(&p);
    assert!(
        text.contains("fn AppStart(_0: String()) -> nothing"),
        "{text}"
    );
    assert!(text.contains("Le Numeric"), "{text}");
    assert!(text.contains("Add Int _1, 1"), "{text}");
    assert!(text.contains("eval Log("), "{text}");
}

#[test]
fn inexact_constant_steps_use_the_run_time_sign_test() {
    // A Float step of 1e-50 is 0 as a Float; a folded Double would say positive.
    let p = program("Dim f As Float\nFor f = 0 To 1 Step 1.0e-50\nLog(f)\nExit\nNext");
    let f = app(&p);
    assert_eq!(compares(f, CmpOp::Gt), 1, "{}", crate::dump(&p));
    assert_eq!(compares(f, CmpOp::Lt), 1);
    // A Long step beyond 2^53 is not folded through a Double.
    let p = program("Dim d As Double\nFor d = 0 To 1 Step 9007199254740993\nExit\nNext");
    assert_eq!(compares(app(&p), CmpOp::Gt), 1);
    // Exact constants still fold.
    let p = program("For i = 1 To 3 Step -1\nNext");
    assert_eq!(compares(app(&p), CmpOp::Gt), 0);
    assert_eq!(compares(app(&p), CmpOp::Ge), 1);
}
