//! End-to-end tests on hand-built MIR programs.

use ferro_mir::*;

use crate::{Outcome, run};

/// Builds one function block by block.
struct Fb {
    name: String,
    params: u32,
    locals: Vec<Local>,
    ret: Ty,
    blocks: Vec<BasicBlock>,
}

impl Fb {
    fn new(name: &str, ret: Ty) -> Fb {
        Fb {
            name: name.into(),
            params: 0,
            locals: Vec::new(),
            ret,
            blocks: vec![BasicBlock {
                stmts: Vec::new(),
                terminator: Terminator::Return(None),
                handler: None,
            }],
        }
    }

    fn param(&mut self, ty: Ty) -> LocalId {
        assert_eq!(
            self.params as usize,
            self.locals.len(),
            "parameters come first"
        );
        self.params += 1;
        self.local(ty)
    }

    fn local(&mut self, ty: Ty) -> LocalId {
        self.locals.push(Local { name: None, ty });
        LocalId(self.locals.len() as u32 - 1)
    }

    fn block(&mut self) -> BlockId {
        self.blocks.push(BasicBlock {
            stmts: Vec::new(),
            terminator: Terminator::Return(None),
            handler: None,
        });
        BlockId(self.blocks.len() as u32 - 1)
    }

    fn stmt(&mut self, b: BlockId, kind: StatementKind) {
        let n = self.blocks[b.0 as usize].stmts.len() as u32;
        self.blocks[b.0 as usize].stmts.push(Statement {
            kind,
            // A distinct range per statement, to check error locations.
            range: TextRange::new((b.0 * 100 + n).into(), (b.0 * 100 + n + 1).into()),
        });
    }

    fn assign(&mut self, b: BlockId, dest: LocalId, value: Rvalue) {
        self.stmt(b, StatementKind::Assign { dest, value });
    }

    /// A new temporary holding `value`.
    fn tmp(&mut self, b: BlockId, ty: Ty, value: Rvalue) -> Operand {
        let t = self.local(ty);
        self.assign(b, t, value);
        Operand::Local(t)
    }

    fn eval(&mut self, b: BlockId, value: Rvalue) {
        self.stmt(b, StatementKind::Eval(value));
    }

    fn log(&mut self, b: BlockId, s: Operand) {
        self.eval(b, builtin(Builtin::Log, None, vec![s]));
    }

    /// `Log(x)` for a value of type `ty`, converted to String first.
    fn log_as(&mut self, b: BlockId, x: Operand, ty: Ty) {
        let s = self.tmp(b, Ty::String, conv(x, ty, Ty::String));
        self.log(b, s);
    }

    fn term(&mut self, b: BlockId, t: Terminator) {
        self.blocks[b.0 as usize].terminator = t;
    }

    fn handler(&mut self, b: BlockId, h: BlockId) {
        self.blocks[b.0 as usize].handler = Some(h);
    }

    fn build(self) -> Function {
        Function {
            name: self.name,
            params: self.params,
            locals: self.locals,
            ret: self.ret,
            blocks: self.blocks,
        }
    }
}

const B0: BlockId = BlockId(0);

#[derive(Default)]
struct Pb {
    records: Vec<Record>,
    globals: Vec<Global>,
    functions: Vec<Option<Function>>,
}

impl Pb {
    /// Reserves an id, for recursive and forward calls.
    fn reserve(&mut self) -> FuncId {
        self.functions.push(None);
        FuncId(self.functions.len() as u32 - 1)
    }

    fn define(&mut self, id: FuncId, f: Fb) {
        self.functions[id.0 as usize] = Some(f.build());
    }

    fn func(&mut self, f: Fb) -> FuncId {
        let id = self.reserve();
        self.define(id, f);
        id
    }

    fn global(&mut self, name: &str, ty: Ty) -> GlobalId {
        self.globals.push(Global {
            name: name.into(),
            ty,
        });
        GlobalId(self.globals.len() as u32 - 1)
    }

    /// A program whose `Process_Globals` is `init` and whose `AppStart` is
    /// built by `main` (local 0 is `Args`).
    fn program(mut self, init: Option<Fb>, main: impl FnOnce(&mut Pb, &mut Fb)) -> Program {
        let init = self.func(init.unwrap_or_else(|| Fb::new("Process_Globals", Ty::Void)));
        let mut entry = Fb::new("AppStart", Ty::Void);
        entry.param(Ty::Array(Box::new(Ty::String), 1));
        let id = self.reserve();
        main(&mut self, &mut entry);
        self.define(id, entry);
        let program = Program {
            records: self.records,
            globals: self.globals,
            functions: self.functions.into_iter().map(Option::unwrap).collect(),
            init,
            entry: id,
        };
        validate(&program).unwrap();
        program
    }
}

fn program(main: impl FnOnce(&mut Pb, &mut Fb)) -> Program {
    Pb::default().program(None, main)
}

fn s(text: &str) -> Operand {
    Operand::Const(Constant::String(text.into()))
}

fn int(i: i32) -> Operand {
    Operand::Const(Constant::Int(i))
}

fn dbl(d: f64) -> Operand {
    Operand::Const(Constant::Double(d))
}

fn boolean(b: bool) -> Operand {
    Operand::Const(Constant::Bool(b))
}

fn null() -> Operand {
    Operand::Const(Constant::Null)
}

fn l(id: LocalId) -> Operand {
    Operand::Local(id)
}

fn conv(value: Operand, from: Ty, to: Ty) -> Rvalue {
    Rvalue::Convert { value, from, to }
}

fn builtin(builtin: Builtin, receiver: Option<Operand>, args: Vec<Operand>) -> Rvalue {
    Rvalue::Builtin {
        builtin,
        receiver,
        args,
    }
}

fn member(b: Builtin, receiver: LocalId, args: Vec<Operand>) -> Rvalue {
    builtin(b, Some(l(receiver)), args)
}

fn arith(op: ArithOp, ty: Ty, left: Operand, right: Operand) -> Rvalue {
    Rvalue::Arith {
        op,
        ty,
        left,
        right,
    }
}

fn cmp(op: CmpOp, kind: CmpKind, left: Operand, right: Operand) -> Rvalue {
    Rvalue::Compare {
        op,
        kind,
        left,
        right,
    }
}

struct Run {
    stdout: String,
    stderr: String,
    outcome: Outcome,
}

fn exec_args(p: &Program, args: &[&str]) -> Run {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let outcome = run(p, &args, &mut out, &mut err);
    Run {
        stdout: String::from_utf8(out).unwrap(),
        stderr: String::from_utf8(err).unwrap(),
        outcome,
    }
}

fn exec(p: &Program) -> Run {
    exec_args(p, &[])
}

/// Runs a program that must end normally and returns its stdout.
fn stdout_of(p: &Program) -> String {
    let r = exec(p);
    assert_eq!(r.outcome.error, None, "stdout so far: {}", r.stdout);
    assert_eq!(r.outcome.exit_code, 0);
    r.stdout
}

/// A conformance test's expected output: its recording, else its
/// hand-derived provisional output (a recording always wins).
fn expected(name: &str) -> String {
    ["expected", "provisional"]
        .iter()
        .find_map(|ext| {
            let path = format!(
                "{}/../../tests/conformance/level0/{name}.{ext}",
                env!("CARGO_MANIFEST_DIR")
            );
            std::fs::read_to_string(path).ok()
        })
        .unwrap_or_else(|| panic!("{name} has no expected output"))
}

#[test]
fn hello_world() {
    let p = program(|_, f| f.log(B0, s("hi")));
    assert_eq!(stdout_of(&p), "hi\n");
}

#[test]
fn log_error_and_log_debug() {
    let p = program(|_, f| {
        f.eval(B0, builtin(Builtin::LogError, None, vec![s("bad")]));
        f.eval(B0, builtin(Builtin::LogDebug, None, vec![s("debug")]));
        f.log(B0, null());
    });
    let r = exec(&p);
    assert_eq!(r.stdout, "null\n");
    assert_eq!(r.stderr, "bad\n");
}

#[test]
fn arguments_reach_app_start() {
    let p = program(|_, f| {
        let n = f.tmp(
            B0,
            Ty::Int,
            builtin(Builtin::ArrayLength, Some(l(LocalId(0))), vec![]),
        );
        f.log_as(B0, n, Ty::Int);
        let a = f.tmp(
            B0,
            Ty::String,
            Rvalue::Index {
                array: l(LocalId(0)),
                indices: vec![int(1)],
            },
        );
        f.log(B0, a);
    });
    assert_eq!(exec_args(&p, &["x", "y"]).stdout, "2\ny\n");
}

/// `tests/conformance/level0/number_to_string.bas`.
#[test]
fn number_to_string_program() {
    let p = program(|_, f| {
        for d in [3.0, 2.5, 1e7, 1234567.5, 0.001, 0.0001] {
            f.log_as(B0, dbl(d), Ty::Double);
        }
        let sum = f.tmp(
            B0,
            Ty::Double,
            arith(ArithOp::Add, Ty::Double, dbl(0.1), dbl(0.2)),
        );
        f.log_as(B0, sum, Ty::Double);
        f.log_as(B0, dbl(123456789012.0), Ty::Double);
        let fl = f.tmp(B0, Ty::Float, conv(dbl(1.1), Ty::Double, Ty::Float));
        f.log_as(B0, fl, Ty::Float);
        f.log_as(B0, int(-42), Ty::Int);
        f.log_as(B0, boolean(true), Ty::Boolean);
    });
    assert_eq!(stdout_of(&p), expected("number_to_string"));
}

/// `tests/conformance/level0/smart_string_format.bas`.
#[test]
fn smart_string_format_program() {
    let p = program(|_, f| {
        let fmt = |f: &mut Fb, format: &str, v: Operand| {
            f.tmp(
                B0,
                Ty::String,
                Rvalue::Format {
                    format: format.into(),
                    value: v,
                },
            )
        };
        let parts: Vec<Operand> = [2, 15, 7]
            .into_iter()
            .map(|x| {
                let o = f.tmp(B0, Ty::Object, conv(int(x), Ty::Int, Ty::Object));
                fmt(f, "2", o)
            })
            .collect();
        let line = f.tmp(
            B0,
            Ty::String,
            Rvalue::Concat(vec![
                s("Remaining time "),
                parts[0].clone(),
                s(":"),
                parts[1].clone(),
                s(":"),
                parts[2].clone(),
            ]),
        );
        f.log(B0, line);
        let q = f.tmp(
            B0,
            Ty::Double,
            arith(ArithOp::Div, Ty::Double, dbl(10.0), dbl(7.0)),
        );
        let q = f.tmp(B0, Ty::Object, conv(q, Ty::Double, Ty::Object));
        let q = fmt(f, "0.3", q);
        let line = f.tmp(B0, Ty::String, Rvalue::Concat(vec![s("10 / 7 = "), q]));
        f.log(B0, line);
        let o = f.tmp(
            B0,
            Ty::Object,
            conv(s("The value is not a number!"), Ty::String, Ty::Object),
        );
        let nan = fmt(f, "1.2", o);
        f.log(B0, nan);
        let p = f.tmp(B0, Ty::Int, arith(ArithOp::Mul, Ty::Int, int(5), int(3)));
        let p = f.tmp(B0, Ty::String, conv(p, Ty::Int, Ty::String));
        let line = f.tmp(B0, Ty::String, Rvalue::Concat(vec![s("5 * 3 = "), p]));
        f.log(B0, line);
        for x in [12345.678, 0.125] {
            let o = f.tmp(B0, Ty::Object, conv(dbl(x), Ty::Double, Ty::Object));
            let t = fmt(f, "1.2", o);
            f.log(B0, t);
        }
        let x = fmt(f, "xml", s("<a & 'b'>"));
        f.log(B0, x);
        f.log(B0, s("No need to escape \"quotes\"!"));
        let n = f.tmp(
            B0,
            Ty::String,
            builtin(
                Builtin::NumberFormat,
                None,
                vec![dbl(12345.6789), int(0), int(2)],
            ),
        );
        f.log(B0, n);
    });
    assert_eq!(stdout_of(&p), expected("smart_string_format"));
}

#[test]
fn number_format_builtins() {
    let p = program(|_, f| {
        let a = f.tmp(
            B0,
            Ty::String,
            builtin(Builtin::NumberFormat, None, vec![dbl(1.0), int(3), int(0)]),
        );
        f.log(B0, a);
        let b = f.tmp(
            B0,
            Ty::String,
            builtin(
                Builtin::NumberFormat2,
                None,
                vec![dbl(12345.67), int(0), int(3), int(3), boolean(false)],
            ),
        );
        f.log(B0, b);
        let o = f.tmp(B0, Ty::Object, conv(int(2), Ty::Int, Ty::Object));
        let c = f.tmp(
            B0,
            Ty::String,
            Rvalue::Format {
                format: "2".into(),
                value: o,
            },
        );
        f.log(B0, c);
    });
    assert_eq!(stdout_of(&p), "001\n12345.670\n02\n");
}

#[test]
fn failed_conversion_is_a_number_format_exception() {
    let p = program(|_, f| {
        f.log(B0, s("start"));
        let n = f.tmp(B0, Ty::Int, conv(s("12x"), Ty::String, Ty::Int));
        f.log_as(B0, n, Ty::Int);
    });
    let r = exec(&p);
    assert_eq!(r.stdout, "start\n");
    assert_eq!(r.outcome.exit_code, 1);
    let e = r.outcome.error.unwrap();
    assert_eq!(e.kind, "java.lang.NumberFormatException");
    assert_eq!(e.message, "For input string: \"12x\"");
    assert_eq!(e.range, TextRange::new(1.into(), 2.into()));
}

#[test]
fn caught_conversion_errors_have_their_kind() {
    // Each conversion inside its own Try; the Catch logs LastException.Message.
    let cases: Vec<(Operand, Ty, Ty)> = vec![
        (s("abc"), Ty::String, Ty::Double),
        (s("yes"), Ty::String, Ty::Boolean),
        (boolean(true), Ty::Object, Ty::Int),
        (int(3), Ty::Object, Ty::Map),
    ];
    let p = program(|_, f| {
        let mut cur = B0;
        for (value, from, to) in cases {
            let body = f.block();
            let catch = f.block();
            let next = f.block();
            f.term(cur, Terminator::Goto(body));
            f.handler(body, catch);
            let t = f.local(to.clone());
            f.assign(body, t, conv(value, from, to));
            f.log(body, s("not reached"));
            f.term(body, Terminator::Goto(next));
            let e = f.tmp(
                catch,
                Ty::Exception,
                builtin(Builtin::LastException, None, vec![]),
            );
            let m = f.tmp(
                catch,
                Ty::String,
                member(Builtin::ExcMessage, local_of(&e), vec![]),
            );
            f.log(catch, m);
            f.term(catch, Terminator::Goto(next));
            cur = next;
        }
    });
    assert_eq!(
        stdout_of(&p),
        "java.lang.NumberFormatException: For input string: \"abc\"\n\
         java.lang.RuntimeException: Cannot parse: yes as boolean\n\
         java.lang.ClassCastException: class java.lang.Boolean cannot be cast to class java.lang.Number\n\
         java.lang.ClassCastException: class java.lang.Integer cannot be cast to class java.util.Map\n"
    );
}

fn local_of(op: &Operand) -> LocalId {
    match op {
        Operand::Local(l) => *l,
        Operand::Const(_) => panic!("not a local"),
    }
}

#[test]
fn integer_arithmetic_wraps_like_java() {
    let p = program(|_, f| {
        let cases = [
            (ArithOp::Add, Ty::Int, int(i32::MAX), int(1)),
            (ArithOp::Mul, Ty::Int, int(65536), int(65536)),
            (ArithOp::Sub, Ty::Int, int(i32::MIN), int(1)),
            (ArithOp::Mod, Ty::Int, int(-1), int(24)),
            (ArithOp::Mod, Ty::Int, int(17), int(5)),
            (ArithOp::Mod, Ty::Int, int(i32::MIN), int(-1)),
            (
                ArithOp::Add,
                Ty::Long,
                Operand::Const(Constant::Long(i64::MAX)),
                Operand::Const(Constant::Long(1)),
            ),
        ];
        for (op, ty, a, b) in cases {
            let r = f.tmp(B0, ty.clone(), arith(op, ty.clone(), a, b));
            f.log_as(B0, r, ty);
        }
        // Byte and Short compute as int, then narrow.
        let b = f.tmp(B0, Ty::Byte, conv(int(127), Ty::Int, Ty::Byte));
        let one = f.tmp(B0, Ty::Byte, conv(int(1), Ty::Int, Ty::Byte));
        let r = f.tmp(B0, Ty::Byte, arith(ArithOp::Add, Ty::Byte, b, one));
        f.log_as(B0, r, Ty::Byte);
        let sh = f.tmp(B0, Ty::Short, conv(int(32767), Ty::Int, Ty::Short));
        let one = f.tmp(B0, Ty::Short, conv(int(1), Ty::Int, Ty::Short));
        let r = f.tmp(B0, Ty::Short, arith(ArithOp::Add, Ty::Short, sh, one));
        f.log_as(B0, r, Ty::Short);
        // Float arithmetic stays in single precision.
        let x = f.tmp(B0, Ty::Float, conv(dbl(0.1), Ty::Double, Ty::Float));
        let y = f.tmp(B0, Ty::Float, conv(dbl(0.2), Ty::Double, Ty::Float));
        let r = f.tmp(B0, Ty::Float, arith(ArithOp::Add, Ty::Float, x, y));
        f.log_as(B0, r, Ty::Float);
        // `/` is Double; division by zero is not an error.
        for (a, b) in [(55.0, 10.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 0.0)] {
            let r = f.tmp(
                B0,
                Ty::Double,
                arith(ArithOp::Div, Ty::Double, dbl(a), dbl(b)),
            );
            f.log_as(B0, r, Ty::Double);
        }
        let r = f.tmp(
            B0,
            Ty::Double,
            arith(ArithOp::Mod, Ty::Double, dbl(-7.5), dbl(2.0)),
        );
        f.log_as(B0, r, Ty::Double);
        let neg = f.tmp(
            B0,
            Ty::Int,
            Rvalue::Neg {
                value: int(i32::MIN),
                ty: Ty::Int,
            },
        );
        f.log_as(B0, neg, Ty::Int);
        // Double → Int truncates toward zero.
        let t = f.tmp(B0, Ty::Int, conv(dbl(-5.5), Ty::Double, Ty::Int));
        f.log_as(B0, t, Ty::Int);
    });
    assert_eq!(
        stdout_of(&p),
        "-2147483648\n0\n2147483647\n-1\n2\n0\n-9223372036854775808\n-128\n-32768\n\
         0.30000001192092896\n5.5\nInfinity\n-Infinity\nNaN\n-1.5\n-2147483648\n-5\n"
    );
}

#[test]
fn integer_mod_by_zero_raises() {
    let p = program(|_, f| {
        let r = f.tmp(B0, Ty::Int, arith(ArithOp::Mod, Ty::Int, int(5), int(0)));
        f.log_as(B0, r, Ty::Int);
    });
    let e = exec(&p).outcome.error.unwrap();
    assert_eq!(e.kind, "java.lang.ArithmeticException");
    assert_eq!(e.message, "/ by zero");
}

#[test]
fn comparisons() {
    let p = program(|_, f| {
        let one_int = f.tmp(B0, Ty::Object, conv(int(1), Ty::Int, Ty::Object));
        let one_dbl = f.tmp(B0, Ty::Object, conv(dbl(1.0), Ty::Double, Ty::Object));
        let other_int = f.tmp(B0, Ty::Object, conv(int(1), Ty::Int, Ty::Object));
        let a = f.tmp(B0, Ty::Object, conv(s("ab"), Ty::String, Ty::Object));
        let ab = f.tmp(B0, Ty::String, Rvalue::Concat(vec![s("a"), s("b")]));
        let b = f.tmp(B0, Ty::Object, conv(ab.clone(), Ty::String, Ty::Object));
        let cases = vec![
            cmp(CmpOp::Eq, CmpKind::Object, one_int.clone(), one_dbl.clone()),
            cmp(CmpOp::Eq, CmpKind::Object, one_int.clone(), other_int),
            cmp(CmpOp::Ne, CmpKind::Object, one_int, one_dbl),
            cmp(CmpOp::Eq, CmpKind::Object, a, b),
            cmp(CmpOp::Eq, CmpKind::Object, null(), null()),
            cmp(CmpOp::Eq, CmpKind::String, s("ab"), ab),
            cmp(CmpOp::Lt, CmpKind::String, s("B"), s("a")),
            cmp(CmpOp::Ge, CmpKind::String, s("abc"), s("abd")),
            cmp(CmpOp::Eq, CmpKind::Numeric, dbl(1.0), dbl(1.0)),
            cmp(CmpOp::Lt, CmpKind::Numeric, dbl(f64::NAN), dbl(1.0)),
            cmp(CmpOp::Ne, CmpKind::Numeric, dbl(f64::NAN), dbl(f64::NAN)),
            cmp(CmpOp::Eq, CmpKind::Boolean, boolean(true), boolean(true)),
        ];
        for c in cases {
            let r = f.tmp(B0, Ty::Boolean, c);
            f.log_as(B0, r, Ty::Boolean);
        }
        // `L = Null` on an uninitialised list, compared as Objects.
        let list = f.tmp(B0, Ty::List, Rvalue::Default(Ty::List));
        let o = f.tmp(B0, Ty::Object, conv(list, Ty::List, Ty::Object));
        let r = f.tmp(B0, Ty::Boolean, cmp(CmpOp::Eq, CmpKind::Object, o, null()));
        f.log_as(B0, r, Ty::Boolean);
    });
    assert_eq!(
        stdout_of(&p),
        "false\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\nfalse\ntrue\nfalse\ntrue\ntrue\ntrue\n"
    );
}

/// `For Each x In collection` lowered by hand: runs `body(f, block, item)`
/// for every element and returns the block after the loop.
fn for_each(
    f: &mut Fb,
    start: BlockId,
    collection: Operand,
    item_ty: Ty,
    body: impl FnOnce(&mut Fb, BlockId, Operand),
) -> BlockId {
    let size = f.tmp(start, Ty::Int, Rvalue::IterSize(collection.clone()));
    let i = f.local(Ty::Int);
    f.assign(start, i, Rvalue::Use(int(0)));
    let head = f.block();
    let body_block = f.block();
    let exit = f.block();
    f.term(start, Terminator::Goto(head));
    let more = f.tmp(
        head,
        Ty::Boolean,
        cmp(CmpOp::Lt, CmpKind::Numeric, l(i), size),
    );
    // Numeric comparisons take Doubles; Int operands are accepted too.
    f.term(
        head,
        Terminator::Branch {
            cond: more,
            then: body_block,
            otherwise: exit,
        },
    );
    let raw = f.tmp(
        body_block,
        Ty::Object,
        Rvalue::IterGet {
            collection,
            index: l(i),
        },
    );
    let item = if item_ty == Ty::Object {
        raw
    } else {
        f.tmp(body_block, item_ty.clone(), conv(raw, Ty::Object, item_ty))
    };
    body(f, body_block, item);
    f.assign(body_block, i, arith(ArithOp::Add, Ty::Int, l(i), int(1)));
    f.term(body_block, Terminator::Goto(head));
    exit
}

/// `tests/conformance/level0/map_order.bas`.
#[test]
fn map_insertion_order_and_reput() {
    let p = program(|_, f| {
        let m = f.local(Ty::Map);
        f.assign(B0, m, Rvalue::Default(Ty::Map));
        f.eval(B0, member(Builtin::MapInitialize, m, vec![]));
        for (k, v) in [("b", 1), ("a", 2), ("c", 3), ("b", 4)] {
            let key = f.tmp(B0, Ty::Object, conv(s(k), Ty::String, Ty::Object));
            let val = f.tmp(B0, Ty::Object, conv(int(v), Ty::Int, Ty::Object));
            f.eval(B0, member(Builtin::MapPut, m, vec![key, val]));
        }
        let keys = f.tmp(B0, Ty::Iterable, member(Builtin::MapKeys, m, vec![]));
        let after = for_each(f, B0, keys, Ty::String, |f, b, k| {
            let key = f.tmp(b, Ty::Object, conv(k.clone(), Ty::String, Ty::Object));
            let v = f.tmp(b, Ty::Object, member(Builtin::MapGet, m, vec![key]));
            let v = f.tmp(b, Ty::String, conv(v, Ty::Object, Ty::String));
            let line = f.tmp(b, Ty::String, Rvalue::Concat(vec![k, s("="), v]));
            f.log(b, line);
        });
        let size = f.tmp(after, Ty::Int, member(Builtin::MapSize, m, vec![]));
        f.log_as(after, size, Ty::Int);
        let big_b = f.tmp(after, Ty::Object, conv(s("B"), Ty::String, Ty::Object));
        let has = f.tmp(
            after,
            Ty::Boolean,
            member(Builtin::MapContainsKey, m, vec![big_b]),
        );
        f.log_as(after, has, Ty::Boolean);
        let z = f.tmp(after, Ty::Object, conv(s("z"), Ty::String, Ty::Object));
        let none = f.tmp(after, Ty::Object, conv(s("none"), Ty::String, Ty::Object));
        let d = f.tmp(
            after,
            Ty::Object,
            member(Builtin::MapGetDefault, m, vec![z, none]),
        );
        f.log_as(after, d, Ty::Object);
    });
    assert_eq!(stdout_of(&p), expected("map_order"));
}

#[test]
fn map_members() {
    let p = program(|_, f| {
        let m = f.tmp(
            B0,
            Ty::Map,
            Rvalue::CreateMap(vec![
                (s("x"), int(1)),
                (int(1), s("int")),
                (dbl(1.0), s("double")),
            ]),
        );
        let m = local_of(&m);
        // Int 1 and Double 1.0 are different keys.
        let one = f.tmp(B0, Ty::Object, conv(int(1), Ty::Int, Ty::Object));
        let v = f.tmp(B0, Ty::Object, member(Builtin::MapGet, m, vec![one]));
        f.log_as(B0, v, Ty::Object);
        let prev = f.tmp(
            B0,
            Ty::Object,
            member(Builtin::MapPut, m, vec![s("x"), int(2)]),
        );
        f.log_as(B0, prev, Ty::Object);
        let prev = f.tmp(
            B0,
            Ty::Object,
            member(Builtin::MapPut, m, vec![s("new"), int(3)]),
        );
        f.log_as(B0, prev, Ty::Object);
        let removed = f.tmp(B0, Ty::Object, member(Builtin::MapRemove, m, vec![int(1)]));
        f.log_as(B0, removed, Ty::Object);
        let key = f.tmp(
            B0,
            Ty::Object,
            member(Builtin::MapGetKeyAt, m, vec![int(1)]),
        );
        f.log_as(B0, key, Ty::Object);
        let value = f.tmp(
            B0,
            Ty::Object,
            member(Builtin::MapGetValueAt, m, vec![int(2)]),
        );
        f.log_as(B0, value, Ty::Object);
        f.log_as(B0, l(m), Ty::Map);
        let values = f.tmp(B0, Ty::Iterable, member(Builtin::MapValues, m, vec![]));
        let after = for_each(f, B0, values, Ty::Object, |f, b, v| {
            f.log_as(b, v, Ty::Object)
        });
        f.eval(after, member(Builtin::MapClear, m, vec![]));
        let n = f.tmp(after, Ty::Int, member(Builtin::MapSize, m, vec![]));
        f.log_as(after, n, Ty::Int);
    });
    assert_eq!(
        stdout_of(&p),
        "int\n1\nnull\nint\n1.0\n3\n(MyMap) {x=2, 1.0=double, new=3}\n2\ndouble\n3\n0\n"
    );
}

#[test]
fn list_members() {
    let p = program(|_, f| {
        let list = f.local(Ty::List);
        f.assign(B0, list, Rvalue::Default(Ty::List));
        let init = f.tmp(
            B0,
            Ty::Boolean,
            member(Builtin::ListIsInitialized, list, vec![]),
        );
        f.log_as(B0, init, Ty::Boolean);
        f.eval(B0, member(Builtin::ListInitialize, list, vec![]));
        for x in [3, 1, 2] {
            f.eval(B0, member(Builtin::ListAdd, list, vec![int(x)]));
        }
        f.eval(
            B0,
            member(Builtin::ListInsertAt, list, vec![int(0), int(10)]),
        );
        f.log_as(B0, l(list), Ty::List);
        f.eval(B0, member(Builtin::ListSort, list, vec![boolean(true)]));
        f.log_as(B0, l(list), Ty::List);
        f.eval(
            B0,
            member(Builtin::ListSortCaseInsensitive, list, vec![boolean(true)]),
        );
        f.log_as(B0, l(list), Ty::List);
        f.eval(B0, member(Builtin::ListSort, list, vec![boolean(false)]));
        f.log_as(B0, l(list), Ty::List);
        f.eval(B0, member(Builtin::ListRemoveAt, list, vec![int(0)]));
        f.eval(B0, member(Builtin::ListSet, list, vec![int(0), s("x")]));
        let g = f.tmp(B0, Ty::Object, member(Builtin::ListGet, list, vec![int(0)]));
        f.log_as(B0, g, Ty::Object);
        let i = f.tmp(
            B0,
            Ty::Int,
            member(Builtin::ListIndexOf, list, vec![int(1)]),
        );
        f.log_as(B0, i, Ty::Int);
        let i = f.tmp(
            B0,
            Ty::Int,
            member(Builtin::ListIndexOf, list, vec![dbl(1.0)]),
        );
        f.log_as(B0, i, Ty::Int);
        let sub = f.tmp(
            B0,
            Ty::List,
            member(Builtin::ListSubList, list, vec![int(1), int(3)]),
        );
        f.log_as(B0, sub, Ty::List);
        let arr = f.tmp(
            B0,
            Ty::Array(Box::new(Ty::String), 1),
            Rvalue::ArrayLiteral {
                elem: Ty::String,
                items: vec![s("b"), s("A"), s("c")],
            },
        );
        let other = f.local(Ty::List);
        f.assign(B0, other, Rvalue::Default(Ty::List));
        let wrapped = f.tmp(
            B0,
            Ty::List,
            conv(arr, Ty::Array(Box::new(Ty::String), 1), Ty::List),
        );
        f.eval(B0, member(Builtin::ListInitialize2, other, vec![wrapped]));
        f.eval(
            B0,
            member(
                Builtin::ListSortCaseInsensitive,
                other,
                vec![boolean(false)],
            ),
        );
        f.eval(B0, member(Builtin::ListAddAll, list, vec![l(other)]));
        f.log_as(B0, l(list), Ty::List);
        let n = f.tmp(B0, Ty::Int, member(Builtin::ListSize, list, vec![]));
        f.log_as(B0, n, Ty::Int);
        f.eval(B0, member(Builtin::ListClear, list, vec![]));
        f.log_as(B0, l(list), Ty::List);
    });
    assert_eq!(
        stdout_of(&p),
        "false\n(ArrayList) [10, 3, 1, 2]\n(ArrayList) [1, 2, 3, 10]\n(ArrayList) [1, 10, 2, 3]\n\
         (ArrayList) [10, 3, 2, 1]\nx\n2\n-1\n(UnmodifiableRandomAccessList) [2, 1]\n(ArrayList) [x, 2, 1, c, b, A]\n6\n\
         (ArrayList) []\n"
    );
}

/// Runs `body` in a Try whose Catch logs `LastException.Message`.
fn try_log(f: &mut Fb, start: BlockId, body: impl FnOnce(&mut Fb, BlockId) -> BlockId) -> BlockId {
    let try_block = f.block();
    let catch = f.block();
    let next = f.block();
    f.term(start, Terminator::Goto(try_block));
    f.handler(try_block, catch);
    let end = body(f, try_block);
    f.term(end, Terminator::Goto(next));
    let e = f.tmp(
        catch,
        Ty::Exception,
        builtin(Builtin::LastException, None, vec![]),
    );
    let m = f.tmp(
        catch,
        Ty::String,
        member(Builtin::ExcMessage, local_of(&e), vec![]),
    );
    f.log(catch, m);
    f.term(catch, Terminator::Goto(next));
    next
}

#[test]
fn list_errors() {
    let p = program(|_, f| {
        let list = f.tmp(B0, Ty::List, Rvalue::Default(Ty::List));
        let list = local_of(&list);
        let mut b = try_log(f, B0, |f, b| {
            f.eval(b, member(Builtin::ListAdd, list, vec![int(1)]));
            b
        });
        f.eval(b, member(Builtin::ListInitialize, list, vec![]));
        f.eval(b, member(Builtin::ListAdd, list, vec![int(1)]));
        b = try_log(f, b, |f, b| {
            f.eval(b, member(Builtin::ListGet, list, vec![int(5)]));
            b
        });
        b = try_log(f, b, |f, b| {
            f.eval(b, member(Builtin::ListInsertAt, list, vec![int(3), int(0)]));
            b
        });
        f.eval(b, member(Builtin::ListAdd, list, vec![dbl(2.0)]));
        b = try_log(f, b, |f, b| {
            f.eval(b, member(Builtin::ListSort, list, vec![boolean(true)]));
            b
        });
        // An array gives a fixed-size list.
        let arr = f.tmp(
            b,
            Ty::Array(Box::new(Ty::Int), 1),
            Rvalue::ArrayLiteral {
                elem: Ty::Int,
                items: vec![int(1), int(2)],
            },
        );
        let fixed = f.local(Ty::List);
        f.assign(b, fixed, Rvalue::Default(Ty::List));
        let wrapped = f.tmp(
            b,
            Ty::List,
            conv(arr, Ty::Array(Box::new(Ty::Int), 1), Ty::List),
        );
        f.eval(b, member(Builtin::ListInitialize2, fixed, vec![wrapped]));
        b = try_log(f, b, |f, b| {
            f.eval(b, member(Builtin::ListAdd, fixed, vec![int(3)]));
            b
        });
        let map = f.tmp(b, Ty::Map, Rvalue::Default(Ty::Map));
        let map = local_of(&map);
        b = try_log(f, b, |f, b| {
            f.eval(b, member(Builtin::MapPut, map, vec![int(1), int(2)]));
            b
        });
        let sb = f.tmp(b, Ty::StringBuilder, Rvalue::Default(Ty::StringBuilder));
        let sb = local_of(&sb);
        try_log(f, b, |f, b| {
            f.eval(b, member(Builtin::SbAppend, sb, vec![s("x")]));
            b
        });
    });
    assert_eq!(
        stdout_of(&p),
        "java.lang.RuntimeException: Object should first be initialized (List).\n\
         java.lang.IndexOutOfBoundsException: Index 5 out of bounds for length 1\n\
         java.lang.IndexOutOfBoundsException: Index: 3, Size: 1\n\
         java.lang.ClassCastException: class java.lang.Double cannot be cast to class java.lang.Integer\n\
         java.lang.UnsupportedOperationException\n\
         java.lang.RuntimeException: Object should first be initialized (Map).\n\
         java.lang.RuntimeException: Object should first be initialized (StringBuilder).\n"
    );
}

#[test]
fn uninitialized_objects_print_their_state() {
    let p = program(|_, f| {
        let list = f.tmp(B0, Ty::List, Rvalue::Default(Ty::List));
        f.log_as(B0, list, Ty::List);
        let map = f.tmp(B0, Ty::Map, Rvalue::Default(Ty::Map));
        f.log_as(B0, map, Ty::Map);
        let e = f.tmp(
            B0,
            Ty::Exception,
            builtin(Builtin::LastException, None, vec![]),
        );
        let init = f.tmp(
            B0,
            Ty::Boolean,
            member(Builtin::ExcIsInitialized, local_of(&e), vec![]),
        );
        f.log_as(B0, init, Ty::Boolean);
    });
    assert_eq!(
        stdout_of(&p),
        "(List) Not initialized\n(Map) Not initialized\nfalse\n"
    );
}

/// A Sub that raises (an uninitialised list), a nested Try in the caller,
/// and an error raised inside a Catch caught by the outer Try.
#[test]
fn nested_handlers_and_propagation() {
    let mut pb = Pb::default();
    let mut fails = Fb::new("Fails", Ty::Int);
    let list = fails.tmp(B0, Ty::List, Rvalue::Default(Ty::List));
    fails.log(B0, s("in Fails"));
    fails.eval(B0, member(Builtin::ListAdd, local_of(&list), vec![int(1)]));
    fails.log(B0, s("not reached"));
    fails.term(B0, Terminator::Return(Some(int(1))));
    let fails = pb.func(fails);
    let p = pb.program(None, |_, f| {
        // Outer Try: blocks outer, outer_catch.
        let outer = f.block();
        let outer_catch = f.block();
        let inner = f.block();
        let inner_catch = f.block();
        let after_inner = f.block();
        let done = f.block();
        f.term(B0, Terminator::Goto(outer));
        f.handler(outer, outer_catch);
        f.log(outer, s("outer try"));
        f.term(outer, Terminator::Goto(inner));
        // Inner Try: a call that raises.
        f.handler(inner, inner_catch);
        let r = f.local(Ty::Int);
        f.assign(
            inner,
            r,
            Rvalue::Call {
                func: fails,
                args: vec![],
            },
        );
        f.log(inner, s("not reached"));
        f.term(inner, Terminator::Goto(after_inner));
        // Inner Catch, still inside the outer Try: logs, then raises.
        f.handler(inner_catch, outer_catch);
        let e = f.tmp(
            inner_catch,
            Ty::Exception,
            builtin(Builtin::LastException, None, vec![]),
        );
        let m = f.tmp(
            inner_catch,
            Ty::String,
            member(Builtin::ExcMessage, local_of(&e), vec![]),
        );
        let line = f.tmp(
            inner_catch,
            Ty::String,
            Rvalue::Concat(vec![s("inner: "), m]),
        );
        f.log(inner_catch, line);
        let bad = f.local(Ty::Int);
        f.assign(inner_catch, bad, conv(s("x"), Ty::String, Ty::Int));
        f.term(inner_catch, Terminator::Goto(after_inner));
        f.handler(after_inner, outer_catch);
        f.log(after_inner, s("not reached"));
        f.term(after_inner, Terminator::Goto(done));
        let e = f.tmp(
            outer_catch,
            Ty::Exception,
            builtin(Builtin::LastException, None, vec![]),
        );
        let m = f.tmp(
            outer_catch,
            Ty::String,
            member(Builtin::ExcMessage, local_of(&e), vec![]),
        );
        let line = f.tmp(
            outer_catch,
            Ty::String,
            Rvalue::Concat(vec![s("outer: "), m]),
        );
        f.log(outer_catch, line);
        f.log_as(outer_catch, e, Ty::Exception);
        f.term(outer_catch, Terminator::Goto(done));
        f.log(done, s("done"));
    });
    assert_eq!(
        stdout_of(&p),
        "outer try\nin Fails\n\
         inner: java.lang.RuntimeException: Object should first be initialized (List).\n\
         outer: java.lang.NumberFormatException: For input string: \"x\"\n\
         (NumberFormatException) java.lang.NumberFormatException: For input string: \"x\"\n\
         done\n"
    );
}

/// `tests/conformance/level0/uncaught_error.bas`.
#[test]
fn uncaught_error_outcome() {
    let p = program(|_, f| {
        f.log(B0, s("before"));
        let m = f.local(Ty::Map);
        f.assign(B0, m, Rvalue::Default(Ty::Map));
        let k = f.tmp(B0, Ty::Object, conv(s("k"), Ty::String, Ty::Object));
        let v = f.tmp(B0, Ty::Object, conv(int(1), Ty::Int, Ty::Object));
        f.eval(B0, member(Builtin::MapPut, m, vec![k, v]));
        f.log(B0, s("not reached"));
    });
    let r = exec(&p);
    assert_eq!(r.stdout, expected("uncaught_error"));
    assert_eq!(r.stderr, "", "run must not print the error");
    assert_eq!(r.outcome.exit_code, 1);
    let e = r.outcome.error.unwrap();
    assert_eq!(e.kind, "java.lang.RuntimeException");
    assert_eq!(e.message, "Object should first be initialized (Map).");
    // The fifth statement of block 0.
    assert_eq!(e.range, TextRange::new(4.into(), 5.into()));
}

#[test]
fn error_in_process_globals_stops_the_program() {
    let mut init = Fb::new("Process_Globals", Ty::Void);
    let t = init.local(Ty::Int);
    init.assign(B0, t, conv(s("?"), Ty::String, Ty::Int));
    let p = Pb::default().program(Some(init), |_, f| f.log(B0, s("not reached")));
    let r = exec(&p);
    assert_eq!(r.stdout, "");
    assert_eq!(
        r.outcome.error.unwrap().kind,
        "java.lang.NumberFormatException"
    );
}

#[test]
fn exit_application_flushes_and_stops() {
    let p = program(|_, f| {
        f.log(B0, s("bye"));
        f.eval(B0, builtin(Builtin::ExitApplication2, None, vec![int(3)]));
        f.log(B0, s("not reached"));
    });
    let r = exec(&p);
    assert_eq!(r.stdout, "bye\n");
    assert_eq!(
        r.outcome,
        Outcome {
            exit_code: 3,
            error: None,
        }
    );
    let p = program(|_, f| {
        // Not caught by a handler.
        let body = f.block();
        let catch = f.block();
        f.term(B0, Terminator::Goto(body));
        f.handler(body, catch);
        f.eval(body, builtin(Builtin::ExitApplication, None, vec![]));
        f.log(catch, s("not reached"));
    });
    assert_eq!(exec(&p).outcome.exit_code, 0);
    assert_eq!(exec(&p).stdout, "");
}

#[test]
fn seeded_random_matches_java() {
    let p = program(|_, f| {
        f.eval(
            B0,
            builtin(
                Builtin::RndSeed,
                None,
                vec![Operand::Const(Constant::Long(42))],
            ),
        );
        for _ in 0..3 {
            let r = f.tmp(
                B0,
                Ty::Int,
                builtin(Builtin::Rnd, None, vec![int(1), int(100)]),
            );
            f.log_as(B0, r, Ty::Int);
        }
    });
    assert_eq!(stdout_of(&p), "63\n49\n64\n");
}

#[test]
fn deep_recursion_is_a_stack_overflow_error() {
    let mut pb = Pb::default();
    let rec = pb.reserve();
    let mut f = Fb::new("Down", Ty::Int);
    let n = f.param(Ty::Int);
    let m = f.tmp(B0, Ty::Int, arith(ArithOp::Add, Ty::Int, l(n), int(1)));
    let r = f.local(Ty::Int);
    f.assign(
        B0,
        r,
        Rvalue::Call {
            func: rec,
            args: vec![m],
        },
    );
    f.term(B0, Terminator::Return(Some(l(r))));
    pb.define(rec, f);
    let p = pb.program(None, |_, f| {
        // A handler does not catch a Java Error.
        let body = f.block();
        let catch = f.block();
        f.term(B0, Terminator::Goto(body));
        f.handler(body, catch);
        let r = f.local(Ty::Int);
        f.assign(
            body,
            r,
            Rvalue::Call {
                func: rec,
                args: vec![int(0)],
            },
        );
        f.log(catch, s("caught"));
    });
    let r = exec(&p);
    assert_eq!(r.stdout, "");
    assert_eq!(r.outcome.exit_code, 1);
    assert_eq!(
        r.outcome.error.unwrap().kind,
        "java.lang.StackOverflowError"
    );
}

#[test]
fn moderate_recursion_works() {
    // Sum(n) = n + Sum(n - 1), 5000 deep.
    let mut pb = Pb::default();
    let sum = pb.reserve();
    let mut f = Fb::new("Sum", Ty::Int);
    let n = f.param(Ty::Int);
    let base = f.block();
    let step = f.block();
    let z = f.tmp(
        B0,
        Ty::Boolean,
        cmp(CmpOp::Eq, CmpKind::Numeric, l(n), int(0)),
    );
    f.term(
        B0,
        Terminator::Branch {
            cond: z,
            then: base,
            otherwise: step,
        },
    );
    f.term(base, Terminator::Return(Some(int(0))));
    let m = f.tmp(step, Ty::Int, arith(ArithOp::Sub, Ty::Int, l(n), int(1)));
    let r = f.local(Ty::Int);
    f.assign(
        step,
        r,
        Rvalue::Call {
            func: sum,
            args: vec![m],
        },
    );
    let t = f.tmp(step, Ty::Int, arith(ArithOp::Add, Ty::Int, l(n), l(r)));
    f.term(step, Terminator::Return(Some(t)));
    pb.define(sum, f);
    let p = pb.program(None, |_, f| {
        let r = f.local(Ty::Int);
        f.assign(
            B0,
            r,
            Rvalue::Call {
                func: sum,
                args: vec![int(5000)],
            },
        );
        f.log_as(B0, l(r), Ty::Int);
    });
    assert_eq!(stdout_of(&p), "12502500\n");
}

/// `tests/conformance/level0/prices_map.bas`, lowered by hand.
#[test]
fn prices_map_program() {
    let mut pb = Pb::default();
    let prices = pb.global("Prices", Ty::Map);
    let p = pb.program(None, |_, f| {
        let m = f.tmp(B0, Ty::Map, Rvalue::Global(prices));
        let m = local_of(&m);
        f.eval(B0, member(Builtin::MapInitialize, m, vec![]));
        for (k, v) in [("coffee", 1.2), ("croissant", 1.5)] {
            let m = f.tmp(B0, Ty::Map, Rvalue::Global(prices));
            let key = f.tmp(B0, Ty::Object, conv(s(k), Ty::String, Ty::Object));
            let val = f.tmp(B0, Ty::Object, conv(dbl(v), Ty::Double, Ty::Object));
            f.eval(B0, member(Builtin::MapPut, local_of(&m), vec![key, val]));
        }
        let total = f.local(Ty::Double);
        f.assign(B0, total, Rvalue::Use(dbl(0.0)));
        let m = f.tmp(B0, Ty::Map, Rvalue::Global(prices));
        let keys = f.tmp(
            B0,
            Ty::Iterable,
            member(Builtin::MapKeys, local_of(&m), vec![]),
        );
        let after = for_each(f, B0, keys, Ty::String, |f, b, k| {
            let m = f.tmp(b, Ty::Map, Rvalue::Global(prices));
            let key = f.tmp(b, Ty::Object, conv(k, Ty::String, Ty::Object));
            let v = f.tmp(
                b,
                Ty::Object,
                member(Builtin::MapGet, local_of(&m), vec![key]),
            );
            let v = f.tmp(b, Ty::Double, conv(v, Ty::Object, Ty::Double));
            f.assign(b, total, arith(ArithOp::Add, Ty::Double, l(total), v));
        });
        let then = f.block();
        let rest = f.block();
        let big = f.tmp(
            after,
            Ty::Boolean,
            cmp(CmpOp::Gt, CmpKind::Numeric, l(total), dbl(2.0)),
        );
        f.term(
            after,
            Terminator::Branch {
                cond: big,
                then,
                otherwise: rest,
            },
        );
        f.log(then, s("Big breakfast"));
        f.term(then, Terminator::Goto(rest));
        let t = f.tmp(rest, Ty::String, conv(l(total), Ty::Double, Ty::String));
        let line = f.tmp(rest, Ty::String, Rvalue::Concat(vec![s("Total: "), t]));
        f.log(rest, line);
    });
    assert_eq!(stdout_of(&p), "Big breakfast\nTotal: 2.7\n");
    assert_eq!(stdout_of(&p), expected("prices_map"));
}

#[test]
fn records() {
    let mut pb = Pb::default();
    let id = RecordId(0);
    let init = pb.reserve();
    pb.records.push(Record {
        name: "Item".into(),
        fields: vec![
            RecordField {
                name: "Name".into(),
                ty: Ty::String,
            },
            RecordField {
                name: "Price".into(),
                ty: Ty::Double,
            },
            RecordField {
                name: "Tags".into(),
                ty: Ty::List,
            },
        ],
        init,
    });
    let mut f = Fb::new("Item.Initialize", Ty::Void);
    let r = f.param(Ty::Record(id));
    f.stmt(
        B0,
        StatementKind::SetField {
            record: l(r),
            field: 0,
            value: s("?"),
        },
    );
    // What the lowering does for the other fields: B4X defaults.
    f.stmt(
        B0,
        StatementKind::SetField {
            record: l(r),
            field: 1,
            value: dbl(0.0),
        },
    );
    let tags = f.tmp(B0, Ty::List, Rvalue::Default(Ty::List));
    f.stmt(
        B0,
        StatementKind::SetField {
            record: l(r),
            field: 2,
            value: tags,
        },
    );
    pb.define(init, f);
    let p = pb.program(None, |_, f| {
        let item = f.tmp(B0, Ty::Record(id), Rvalue::Default(Ty::Record(id)));
        let item_id = local_of(&item);
        let ok = f.tmp(
            B0,
            Ty::Boolean,
            member(Builtin::RecordIsInitialized, item_id, vec![]),
        );
        f.log_as(B0, ok, Ty::Boolean);
        // Before Initialize the fields hold Java defaults and can be used.
        f.stmt(
            B0,
            StatementKind::SetField {
                record: item.clone(),
                field: 1,
                value: dbl(1.0),
            },
        );
        let name = f.tmp(
            B0,
            Ty::String,
            Rvalue::Field {
                record: item.clone(),
                field: 0,
            },
        );
        f.log(B0, name);
        f.log_as(B0, item.clone(), Ty::Record(id));
        // A List field is a Java null: calling a member raises.
        let tags = f.tmp(
            B0,
            Ty::List,
            Rvalue::Field {
                record: item.clone(),
                field: 2,
            },
        );
        let b = try_log(f, B0, |f, b| {
            f.eval(b, member(Builtin::ListSize, local_of(&tags), vec![]));
            b
        });
        f.eval(b, member(Builtin::RecordInitialize, item_id, vec![]));
        let ok = f.tmp(
            b,
            Ty::Boolean,
            member(Builtin::RecordIsInitialized, item_id, vec![]),
        );
        f.log_as(b, ok, Ty::Boolean);
        let name = f.tmp(
            b,
            Ty::String,
            Rvalue::Field {
                record: item.clone(),
                field: 0,
            },
        );
        f.log(b, name);
        f.stmt(
            b,
            StatementKind::SetField {
                record: item.clone(),
                field: 1,
                value: dbl(2.5),
            },
        );
        f.log_as(b, item.clone(), Ty::Record(id));
        // SortType by Price.
        let list = f.local(Ty::List);
        f.assign(b, list, Rvalue::Default(Ty::List));
        f.eval(b, member(Builtin::ListInitialize, list, vec![]));
        for (name, price) in [("b", 3.0), ("a", 1.0), ("c", 2.0)] {
            let it = f.tmp(b, Ty::Record(id), Rvalue::Default(Ty::Record(id)));
            f.eval(b, member(Builtin::RecordInitialize, local_of(&it), vec![]));
            f.stmt(
                b,
                StatementKind::SetField {
                    record: it.clone(),
                    field: 0,
                    value: s(name),
                },
            );
            f.stmt(
                b,
                StatementKind::SetField {
                    record: it.clone(),
                    field: 1,
                    value: dbl(price),
                },
            );
            let o = f.tmp(b, Ty::Object, conv(it, Ty::Record(id), Ty::Object));
            f.eval(b, member(Builtin::ListAdd, list, vec![o]));
        }
        f.eval(
            b,
            member(Builtin::ListSortType, list, vec![s("Price"), boolean(true)]),
        );
        let after = for_each(f, b, l(list), Ty::Record(id), |f, b, it| {
            let n = f.tmp(
                b,
                Ty::String,
                Rvalue::Field {
                    record: it,
                    field: 0,
                },
            );
            f.log(b, n);
        });
        f.eval(
            after,
            member(
                Builtin::ListSortTypeCaseInsensitive,
                list,
                vec![s("Name"), boolean(false)],
            ),
        );
        let first = f.tmp(
            after,
            Ty::Object,
            member(Builtin::ListGet, list, vec![int(0)]),
        );
        let first = f.tmp(
            after,
            Ty::Record(id),
            conv(first, Ty::Object, Ty::Record(id)),
        );
        let n = f.tmp(
            after,
            Ty::String,
            Rvalue::Field {
                record: first,
                field: 0,
            },
        );
        f.log(after, n);
    });
    assert_eq!(
        stdout_of(&p),
        "false\nnull\n[IsInitialized=false, Name=null, Price=1.0, Tags=null]\n\
         java.lang.NullPointerException\ntrue\n?\n\
         [IsInitialized=true, Name=?, Price=2.5, Tags=(List) Not initialized]\na\nc\nb\nc\n"
    );
}

#[test]
fn arrays() {
    let p = program(|_, f| {
        let ty = Ty::Array(Box::new(Ty::Int), 2);
        let m = f.tmp(
            B0,
            ty,
            Rvalue::NewArray {
                elem: Ty::Int,
                dims: vec![int(3), int(4)],
            },
        );
        f.stmt(
            B0,
            StatementKind::SetIndex {
                array: m.clone(),
                indices: vec![int(2), int(3)],
                value: int(7),
            },
        );
        let v = f.tmp(
            B0,
            Ty::Int,
            Rvalue::Index {
                array: m.clone(),
                indices: vec![int(2), int(3)],
            },
        );
        f.log_as(B0, v, Ty::Int);
        let z = f.tmp(
            B0,
            Ty::Int,
            Rvalue::Index {
                array: m.clone(),
                indices: vec![int(0), int(0)],
            },
        );
        f.log_as(B0, z, Ty::Int);
        let n = f.tmp(
            B0,
            Ty::Int,
            builtin(Builtin::ArrayLength, Some(m.clone()), vec![]),
        );
        f.log_as(B0, n, Ty::Int);
        let b = try_log(f, B0, |f, b| {
            f.eval(
                b,
                Rvalue::Index {
                    array: m.clone(),
                    indices: vec![int(1), int(4)],
                },
            );
            b
        });
        let empty = f.tmp(
            b,
            Ty::Array(Box::new(Ty::String), 1),
            Rvalue::Default(Ty::Array(Box::new(Ty::String), 1)),
        );
        let n = f.tmp(
            b,
            Ty::Int,
            builtin(Builtin::ArrayLength, Some(empty), vec![]),
        );
        f.log_as(b, n, Ty::Int);
        let lit = f.tmp(
            b,
            Ty::Array(Box::new(Ty::Int), 1),
            Rvalue::ArrayLiteral {
                elem: Ty::Int,
                items: vec![int(1), int(2), int(3)],
            },
        );
        let total = f.local(Ty::Int);
        let after = for_each(f, b, lit, Ty::Object, |f, b, x| {
            let x = f.tmp(b, Ty::Int, conv(x, Ty::Object, Ty::Int));
            f.assign(b, total, arith(ArithOp::Add, Ty::Int, l(total), x));
        });
        f.log_as(after, l(total), Ty::Int);
    });
    assert_eq!(
        stdout_of(&p),
        "7\n0\n3\njava.lang.ArrayIndexOutOfBoundsException: Index 4 out of bounds for length 4\n0\n6\n"
    );
}

#[test]
fn string_members() {
    let p = program(|_, f| {
        let text = f.tmp(B0, Ty::String, Rvalue::Use(s("  Hello, World  ")));
        let t = local_of(&text);
        let trimmed = f.tmp(B0, Ty::String, member(Builtin::StrTrim, t, vec![]));
        let tr = local_of(&trimmed);
        let calls: Vec<(Builtin, Vec<Operand>, Ty)> = vec![
            (Builtin::StrLength, vec![], Ty::Int),
            (Builtin::StrCharAt, vec![int(1)], Ty::Char),
            (Builtin::StrIndexOf, vec![s("o")], Ty::Int),
            (Builtin::StrIndexOf2, vec![s("o"), int(5)], Ty::Int),
            (Builtin::StrLastIndexOf, vec![s("o")], Ty::Int),
            (Builtin::StrSubString, vec![int(7)], Ty::String),
            (Builtin::StrSubString2, vec![int(0), int(5)], Ty::String),
            (Builtin::StrToUpperCase, vec![], Ty::String),
            (Builtin::StrToLowerCase, vec![], Ty::String),
            (Builtin::StrReplace, vec![s("l"), s("L")], Ty::String),
            (Builtin::StrContains, vec![s("World")], Ty::Boolean),
            (Builtin::StrStartsWith, vec![s("Hell")], Ty::Boolean),
            (Builtin::StrEndsWith, vec![s("x")], Ty::Boolean),
            (
                Builtin::StrEqualsIgnoreCase,
                vec![s("HELLO, WORLD")],
                Ty::Boolean,
            ),
            (Builtin::StrCompareTo, vec![s("Help")], Ty::Int),
        ];
        for (b, args, ty) in calls {
            let r = f.tmp(B0, ty.clone(), member(b, tr, args));
            f.log_as(B0, r, ty);
        }
        let n = f.tmp(B0, Ty::Int, member(Builtin::StrLength, t, vec![]));
        f.log_as(B0, n, Ty::Int);
        let next = try_log(f, B0, |f, b| {
            f.eval(b, member(Builtin::StrSubString2, tr, vec![int(3), int(99)]));
            b
        });
        let next = try_log(f, next, |f, b| {
            f.eval(b, member(Builtin::StrCharAt, tr, vec![int(12)]));
            b
        });
        // StringBuilder.
        let sb = f.local(Ty::StringBuilder);
        f.assign(next, sb, Rvalue::Default(Ty::StringBuilder));
        f.eval(next, member(Builtin::SbInitialize, sb, vec![]));
        f.eval(next, member(Builtin::SbAppend, sb, vec![s("abc")]));
        f.eval(next, member(Builtin::SbAppend, sb, vec![s("def")]));
        f.eval(next, member(Builtin::SbInsert, sb, vec![int(0), s(">")]));
        f.eval(next, member(Builtin::SbRemove, sb, vec![int(2), int(4)]));
        let r = f.tmp(next, Ty::String, member(Builtin::SbToString, sb, vec![]));
        f.log(next, r);
        let n = f.tmp(next, Ty::Int, member(Builtin::SbLength, sb, vec![]));
        f.log_as(next, n, Ty::Int);
        f.log_as(next, l(sb), Ty::StringBuilder);
        // Chr / Asc and Bit.
        let c = f.tmp(next, Ty::Char, builtin(Builtin::Chr, None, vec![int(65)]));
        let a = f.tmp(next, Ty::Int, builtin(Builtin::Asc, None, vec![c.clone()]));
        f.log_as(next, c, Ty::Char);
        f.log_as(next, a, Ty::Int);
        let h = f.tmp(
            next,
            Ty::String,
            builtin(Builtin::BitToHexString, None, vec![int(-1)]),
        );
        f.log(next, h);
        let x = f.tmp(
            next,
            Ty::Int,
            builtin(
                Builtin::BitUnsignedShiftRight,
                None,
                vec![int(-16), int(28)],
            ),
        );
        f.log_as(next, x, Ty::Int);
        let x = f.tmp(
            next,
            Ty::Int,
            builtin(Builtin::BitParseInt, None, vec![s("ff"), int(16)]),
        );
        f.log_as(next, x, Ty::Int);
    });
    assert_eq!(
        stdout_of(&p),
        "12\ne\n4\n8\n8\nWorld\nHello\nHELLO, WORLD\nhello, world\nHeLLo, WorLd\ntrue\ntrue\nfalse\n\
         true\n-4\n16\n\
         java.lang.StringIndexOutOfBoundsException: begin 3, end 99, length 12\n\
         java.lang.StringIndexOutOfBoundsException: Index 12 out of bounds for length 12\n\
         >adef\n5\n(StringBuilder) >adef\nA\n65\nffffffff\n15\n255\n"
    );
}

#[test]
fn math_builtins() {
    let p = program(|_, f| {
        let calls: Vec<(Builtin, Vec<Operand>, Ty)> = vec![
            (Builtin::Round, vec![dbl(-2.5)], Ty::Long),
            (Builtin::Round, vec![dbl(2.5)], Ty::Long),
            (Builtin::Round2, vec![dbl(1.23456), int(2)], Ty::Double),
            (Builtin::Floor, vec![dbl(-1.5)], Ty::Double),
            (Builtin::Ceil, vec![dbl(1.2)], Ty::Double),
            (Builtin::Sqrt, vec![dbl(16.0)], Ty::Double),
            (Builtin::Power, vec![dbl(2.0), dbl(10.0)], Ty::Double),
            (Builtin::Max, vec![dbl(1.0), dbl(2.0)], Ty::Double),
            (Builtin::Abs, vec![dbl(-3.0)], Ty::Double),
            (Builtin::SinD, vec![dbl(90.0)], Ty::Double),
            (Builtin::Logarithm, vec![dbl(8.0), dbl(2.0)], Ty::Double),
            (Builtin::Pi, vec![], Ty::Double),
            (Builtin::IsNumber, vec![s("1e5")], Ty::Boolean),
            (Builtin::IsNumber, vec![s("abc")], Ty::Boolean),
        ];
        for (b, args, ty) in calls {
            let r = f.tmp(B0, ty.clone(), builtin(b, None, args));
            f.log_as(B0, r, ty);
        }
        let o = f.tmp(B0, Ty::Object, conv(int(1), Ty::Int, Ty::Object));
        let t = f.tmp(B0, Ty::String, builtin(Builtin::GetType, None, vec![o]));
        f.log(B0, t);
        let o = f.tmp(B0, Ty::Object, conv(s("x"), Ty::String, Ty::Object));
        let t = f.tmp(B0, Ty::String, builtin(Builtin::GetType, None, vec![o]));
        f.log(B0, t);
        let crlf = f.tmp(B0, Ty::String, builtin(Builtin::Crlf, None, vec![]));
        let q = f.tmp(B0, Ty::String, builtin(Builtin::Quote, None, vec![]));
        let line = f.tmp(
            B0,
            Ty::String,
            Rvalue::Concat(vec![q.clone(), s("a"), q, crlf]),
        );
        f.log(B0, line);
    });
    assert_eq!(
        stdout_of(&p),
        "-2\n3\n1.23\n-2\n2\n4\n1024\n2\n3\n1\n3\n3.141592653589793\ntrue\nfalse\n\
         java.lang.Integer\njava.lang.String\n\"a\"\n\n"
    );
}

#[test]
fn locals_are_reset_on_every_call_and_dim() {
    // Calling a Sub twice: its List local starts uninitialised each time.
    let mut pb = Pb::default();
    let mut f = Fb::new("Check", Ty::Boolean);
    let list = f.local(Ty::List);
    let was = f.tmp(
        B0,
        Ty::Boolean,
        member(Builtin::ListIsInitialized, list, vec![]),
    );
    f.eval(B0, member(Builtin::ListInitialize, list, vec![]));
    f.term(B0, Terminator::Return(Some(was)));
    let check = pb.func(f);
    let p = pb.program(None, |_, f| {
        for _ in 0..2 {
            let r = f.local(Ty::Boolean);
            f.assign(
                B0,
                r,
                Rvalue::Call {
                    func: check,
                    args: vec![],
                },
            );
            f.log_as(B0, l(r), Ty::Boolean);
        }
    });
    assert_eq!(stdout_of(&p), "false\nfalse\n");
}

#[test]
fn invalid_program_is_reported_not_run() {
    let mut p = program(|_, f| f.log(B0, s("hi")));
    p.functions[1].blocks[0].terminator = Terminator::Goto(BlockId(9));
    let r = exec(&p);
    assert_eq!(r.stdout, "");
    assert_eq!(r.outcome.error.unwrap().kind, "ferro.InternalError");
}

// Regression tests for the review of the first version.

fn str_array(f: &mut Fb, b: BlockId, items: &[&str]) -> Operand {
    f.tmp(
        b,
        Ty::Array(Box::new(Ty::String), 1),
        Rvalue::ArrayLiteral {
            elem: Ty::String,
            items: items.iter().map(|x| s(x)).collect(),
        },
    )
}

fn obj(f: &mut Fb, b: BlockId, v: Operand, from: Ty) -> Operand {
    f.tmp(b, Ty::Object, conv(v, from, Ty::Object))
}

/// Runs `body` in a Try: logs `ok` when it completes, else the message.
fn try_ok(f: &mut Fb, start: BlockId, body: impl FnOnce(&mut Fb, BlockId)) -> BlockId {
    try_log(f, start, |f, b| {
        body(f, b);
        f.log(b, s("ok"));
        b
    })
}

/// A `String()` seen as an `Object()` keeps its runtime component type:
/// storing an Int raises `ArrayStoreException` before any change, through
/// the array and through a fixed-size list sharing it.
#[test]
fn array_store_checks_the_runtime_component_type() {
    let p = program(|_, f| {
        let arr = str_array(f, B0, &["a", "b"]);
        let o = obj(f, B0, arr.clone(), Ty::Array(Box::new(Ty::String), 1));
        let objs = f.tmp(
            B0,
            Ty::Array(Box::new(Ty::Object), 1),
            conv(o, Ty::Object, Ty::Array(Box::new(Ty::Object), 1)),
        );
        let one = obj(f, B0, int(1), Ty::Int);
        let b = try_ok(f, B0, |f, b| {
            f.stmt(
                b,
                StatementKind::SetIndex {
                    array: objs.clone(),
                    indices: vec![int(0)],
                    value: one.clone(),
                },
            );
        });
        let x = obj(f, b, s("x"), Ty::String);
        let b = try_ok(f, b, |f, b| {
            f.stmt(
                b,
                StatementKind::SetIndex {
                    array: objs.clone(),
                    indices: vec![int(1)],
                    value: x,
                },
            );
        });
        let list = f.local(Ty::List);
        f.assign(b, list, Rvalue::Default(Ty::List));
        let wrapped = f.tmp(
            b,
            Ty::List,
            conv(arr.clone(), Ty::Array(Box::new(Ty::String), 1), Ty::List),
        );
        f.eval(b, member(Builtin::ListInitialize2, list, vec![wrapped]));
        let b = try_ok(f, b, |f, b| {
            f.eval(b, member(Builtin::ListSet, list, vec![int(0), one.clone()]));
        });
        let y = obj(f, b, s("y"), Ty::String);
        f.eval(b, member(Builtin::ListSet, list, vec![int(0), y]));
        let joined = f.tmp(
            b,
            Ty::List,
            conv(arr, Ty::Array(Box::new(Ty::String), 1), Ty::List),
        );
        f.log_as(b, joined, Ty::List);
    });
    assert_eq!(
        stdout_of(&p),
        "java.lang.ArrayStoreException: java.lang.Integer\nok\n\
         java.lang.ArrayStoreException: java.lang.Integer\n(ArrayList) [y, x]\n"
    );
}

/// Object → array checks rank, element type and primitive vs reference,
/// with reference-array covariance.
#[test]
fn object_to_array_checks_the_whole_type() {
    let arr = |e: Ty, r: u8| Ty::Array(Box::new(e), r);
    let p = program(|_, f| {
        let ints = f.tmp(
            B0,
            arr(Ty::Int, 1),
            Rvalue::ArrayLiteral {
                elem: Ty::Int,
                items: vec![int(1)],
            },
        );
        let grid = f.tmp(
            B0,
            arr(Ty::Int, 2),
            Rvalue::NewArray {
                elem: Ty::Int,
                dims: vec![int(2), int(2)],
            },
        );
        let strs = str_array(f, B0, &["a"]);
        let cases = [
            (ints.clone(), arr(Ty::Int, 1), arr(Ty::Object, 1)),
            (ints.clone(), arr(Ty::Int, 1), arr(Ty::Long, 1)),
            (grid.clone(), arr(Ty::Int, 2), arr(Ty::Int, 1)),
            (grid, arr(Ty::Int, 2), arr(Ty::Object, 1)),
            (strs.clone(), arr(Ty::String, 1), arr(Ty::Object, 1)),
            (strs, arr(Ty::String, 1), arr(Ty::String, 2)),
            (ints, arr(Ty::Int, 1), arr(Ty::Int, 1)),
        ];
        let mut b = B0;
        for (v, from, to) in cases {
            let o = obj(f, b, v, from);
            b = try_ok(f, b, |f, b| {
                let t = f.local(to.clone());
                f.assign(b, t, conv(o, Ty::Object, to));
            });
        }
    });
    assert_eq!(
        stdout_of(&p),
        "java.lang.ClassCastException: class [I cannot be cast to class [Ljava.lang.Object;\n\
         java.lang.ClassCastException: class [I cannot be cast to class [J\n\
         java.lang.ClassCastException: class [[I cannot be cast to class [I\n\
         ok\nok\n\
         java.lang.ClassCastException: class [Ljava.lang.String; cannot be cast to class [[Ljava.lang.String;\n\
         ok\n"
    );
}

/// `a.Add(a)`, `b.Add(b)`: `a = b` recurses like Java's `ArrayList.equals`
/// and ends in `StackOverflowError` instead of crashing; `a = a` is true.
#[test]
fn equality_of_self_containing_lists_is_bounded() {
    let build = |compare: bool| {
        program(|_, f| {
            let mut objs = Vec::new();
            for _ in 0..2 {
                let list = f.local(Ty::List);
                f.assign(B0, list, Rvalue::Default(Ty::List));
                f.eval(B0, member(Builtin::ListInitialize, list, vec![]));
                let o = obj(f, B0, l(list), Ty::List);
                f.eval(B0, member(Builtin::ListAdd, list, vec![o.clone()]));
                objs.push((list, o));
            }
            let (a, oa) = objs[0].clone();
            let (_, ob) = objs[1].clone();
            let same = f.tmp(
                B0,
                Ty::Boolean,
                cmp(CmpOp::Eq, CmpKind::Object, oa.clone(), oa.clone()),
            );
            f.log_as(B0, same, Ty::Boolean);
            f.log_as(B0, l(a), Ty::List);
            if compare {
                let r = f.tmp(B0, Ty::Boolean, cmp(CmpOp::Eq, CmpKind::Object, oa, ob));
                f.log_as(B0, r, Ty::Boolean);
            } else {
                let r = f.tmp(B0, Ty::Int, member(Builtin::ListIndexOf, a, vec![ob]));
                f.log_as(B0, r, Ty::Int);
            }
        })
    };
    for compare in [true, false] {
        let r = exec(&build(compare));
        assert_eq!(r.stdout, "true\n(ArrayList) [(this Collection)]\n");
        assert_eq!(
            r.outcome.error.unwrap().kind,
            "java.lang.StackOverflowError"
        );
    }
}

#[test]
fn initialize2_with_an_uninitialised_list_raises_first() {
    let p = program(|_, f| {
        let dest = f.local(Ty::List);
        f.assign(B0, dest, Rvalue::Default(Ty::List));
        f.eval(B0, member(Builtin::ListInitialize, dest, vec![]));
        f.eval(B0, member(Builtin::ListAdd, dest, vec![int(1)]));
        let src = f.tmp(B0, Ty::List, Rvalue::Default(Ty::List));
        let b = try_ok(f, B0, |f, b| {
            f.eval(b, member(Builtin::ListInitialize2, dest, vec![src]));
        });
        f.log_as(b, l(dest), Ty::List);
    });
    assert_eq!(
        stdout_of(&p),
        "java.lang.RuntimeException: Object should first be initialized (List).\n\
         (ArrayList) [1]\n"
    );
}

/// `SubList` is `Collections.unmodifiableList(parent.subList(a, b))`.
#[test]
fn sub_lists_are_read_only_views() {
    let mut pb = Pb::default();
    let id = RecordId(0);
    let init = pb.func(Fb::new("T.Initialize", Ty::Void));
    // `init` takes the record as its parameter.
    pb.functions[init.0 as usize].as_mut().unwrap().params = 1;
    pb.functions[init.0 as usize]
        .as_mut()
        .unwrap()
        .locals
        .push(Local {
            name: None,
            ty: Ty::Record(id),
        });
    pb.records.push(Record {
        name: "T".into(),
        fields: vec![RecordField {
            name: "N".into(),
            ty: Ty::Int,
        }],
        init,
    });
    let p = pb.program(None, |_, f| {
        let parent = f.local(Ty::List);
        f.assign(B0, parent, Rvalue::Default(Ty::List));
        f.eval(B0, member(Builtin::ListInitialize, parent, vec![]));
        for x in [1, 2, 3] {
            f.eval(B0, member(Builtin::ListAdd, parent, vec![int(x)]));
        }
        // Empty views: Clear and Sort always refuse, SortType returns early.
        let empty = f.tmp(
            B0,
            Ty::List,
            member(Builtin::ListSubList, parent, vec![int(1), int(1)]),
        );
        let empty = local_of(&empty);
        let mut b = B0;
        for (op, args) in [
            (Builtin::ListClear, vec![]),
            (Builtin::ListSort, vec![boolean(true)]),
            (Builtin::ListSortCaseInsensitive, vec![boolean(true)]),
            (Builtin::ListSortType, vec![s("N"), boolean(true)]),
            (
                Builtin::ListSortTypeCaseInsensitive,
                vec![s("N"), boolean(true)],
            ),
        ] {
            b = try_ok(f, b, |f, b| f.eval(b, member(op, empty, args)));
        }
        // An empty fixed-size list can still be cleared.
        let fixed = f.local(Ty::List);
        f.assign(b, fixed, Rvalue::Default(Ty::List));
        let none = str_array(f, b, &[]);
        let wrapped = f.tmp(
            b,
            Ty::List,
            conv(none, Ty::Array(Box::new(Ty::String), 1), Ty::List),
        );
        f.eval(b, member(Builtin::ListInitialize2, fixed, vec![wrapped]));
        b = try_ok(f, b, |f, b| {
            f.eval(b, member(Builtin::ListClear, fixed, vec![]))
        });
        // A live view: Set on the parent shows through.
        let child = f.tmp(
            b,
            Ty::List,
            member(Builtin::ListSubList, parent, vec![int(0), int(2)]),
        );
        let child = local_of(&child);
        f.eval(b, member(Builtin::ListSet, parent, vec![int(0), s("new")]));
        let first = f.tmp(b, Ty::Object, member(Builtin::ListGet, child, vec![int(0)]));
        f.log_as(b, first, Ty::Object);
        f.log_as(b, l(child), Ty::List);
        let o = obj(f, b, l(child), Ty::List);
        let t = f.tmp(b, Ty::String, builtin(Builtin::GetType, None, vec![o]));
        f.log(b, t);
        b = try_ok(f, b, |f, b| {
            f.eval(b, member(Builtin::ListSet, child, vec![int(0), int(9)]));
        });
        // A structural change of the parent invalidates the view.
        f.eval(b, member(Builtin::ListAdd, parent, vec![int(4)]));
        b = try_ok(f, b, |f, b| {
            f.eval(b, member(Builtin::ListGet, child, vec![int(0)]));
        });
        b = try_ok(f, b, |f, b| {
            f.eval(b, member(Builtin::ListSize, child, vec![]));
        });
        try_ok(f, b, |f, b| f.log_as(b, l(child), Ty::List));
    });
    assert_eq!(
        stdout_of(&p),
        "java.lang.UnsupportedOperationException\n\
         java.lang.UnsupportedOperationException\n\
         java.lang.UnsupportedOperationException\n\
         ok\nok\nok\n\
         new\n(UnmodifiableRandomAccessList) [new, 2]\n\
         java.util.Collections$UnmodifiableRandomAccessList\n\
         java.lang.UnsupportedOperationException\n\
         java.util.ConcurrentModificationException\n\
         java.util.ConcurrentModificationException\n\
         java.util.ConcurrentModificationException\n"
    );
}

/// `SortTypeCaseInsensitive` compares `String.valueOf(field)` ("null" for
/// Null); `SortCaseInsensitive` calls `toString()` and raises on Null.
#[test]
fn case_insensitive_sorts_and_null() {
    let mut pb = Pb::default();
    let id = RecordId(0);
    let init = pb.reserve();
    pb.records.push(Record {
        name: "T".into(),
        fields: vec![RecordField {
            name: "Name".into(),
            ty: Ty::String,
        }],
        init,
    });
    let mut f = Fb::new("T.Initialize", Ty::Void);
    f.param(Ty::Record(id));
    pb.define(init, f);
    let p = pb.program(None, |_, f| {
        let list = f.local(Ty::List);
        f.assign(B0, list, Rvalue::Default(Ty::List));
        f.eval(B0, member(Builtin::ListInitialize, list, vec![]));
        for name in [Some("b"), None, Some("A"), Some("o")] {
            // Not initialised: Name is a Java null until set.
            let t = f.tmp(B0, Ty::Record(id), Rvalue::Default(Ty::Record(id)));
            if let Some(name) = name {
                f.stmt(
                    B0,
                    StatementKind::SetField {
                        record: t.clone(),
                        field: 0,
                        value: s(name),
                    },
                );
            }
            let o = obj(f, B0, t, Ty::Record(id));
            f.eval(B0, member(Builtin::ListAdd, list, vec![o]));
        }
        f.eval(
            B0,
            member(
                Builtin::ListSortTypeCaseInsensitive,
                list,
                vec![s("Name"), boolean(true)],
            ),
        );
        let after = for_each(f, B0, l(list), Ty::Record(id), |f, b, t| {
            let n = f.tmp(
                b,
                Ty::String,
                Rvalue::Field {
                    record: t,
                    field: 0,
                },
            );
            f.log(b, n);
        });
        let names = f.local(Ty::List);
        f.assign(after, names, Rvalue::Default(Ty::List));
        f.eval(after, member(Builtin::ListInitialize, names, vec![]));
        f.eval(after, member(Builtin::ListAdd, names, vec![s("x")]));
        f.eval(after, member(Builtin::ListAdd, names, vec![null()]));
        try_ok(f, after, |f, b| {
            f.eval(
                b,
                member(Builtin::ListSortCaseInsensitive, names, vec![boolean(true)]),
            );
        });
    });
    assert_eq!(
        stdout_of(&p),
        "A\nb\nnull\no\njava.lang.NullPointerException\n"
    );
}

/// Output still buffered when the machine goes away (also when it unwinds)
/// is written out.
#[test]
fn buffered_output_is_flushed_on_drop() {
    let p = program(|_, _| {});
    let (mut out, mut err) = (Vec::new(), Vec::new());
    {
        let mut m = crate::machine::Machine::new(&p, &mut out, &mut err);
        m.out.log(&crate::strings::to_utf16("kept"));
    }
    assert_eq!(out, b"kept\n");
}

/// Long lines are encoded and written piece by piece, with the same bytes.
#[test]
fn long_lines_are_written_in_pieces() {
    let p = program(|_, _| {});
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut text: Vec<u16> = "é€".encode_utf16().collect::<Vec<_>>().repeat(50_000);
    text.push(0xD800);
    text.extend("😀".encode_utf16());
    {
        let mut m = crate::machine::Machine::new(&p, &mut out, &mut err);
        m.out.log(&text);
        m.out.log_error(&text);
    }
    let expected = format!("{}?😀\n", "é€".repeat(50_000));
    assert_eq!(String::from_utf8(out).unwrap(), expected);
    assert_eq!(String::from_utf8(err).unwrap(), expected);
}

/// A line longer than the buffer is written out whole, line feed included,
/// before `Log` returns.
#[test]
fn long_lines_are_flushed_whole() {
    #[derive(Clone, Default)]
    struct Shared(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
    impl std::io::Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let p = program(|_, _| {});
    let (mut out, mut err) = (Shared::default(), Vec::new());
    let seen = out.clone();
    let mut m = crate::machine::Machine::new(&p, &mut out, &mut err);
    m.out.log(&vec![b'x' as u16; 70_000]);
    assert_eq!(seen.0.borrow().len(), 70_001);
    assert_eq!(seen.0.borrow().last(), Some(&b'\n'));
    m.out.log(&crate::strings::to_utf16("short"));
    assert_eq!(seen.0.borrow().len(), 70_001, "short lines stay buffered");
}
