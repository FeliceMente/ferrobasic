//! The execution engine: an explicit frame stack (no Rust recursion per
//! BASIC call), block-level exception handlers, buffered output.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

use ferro_mir::{
    ArithOp, Builtin, CmpKind, CmpOp, Constant, FuncId, Function, LocalId, Operand, Program,
    Rvalue, StatementKind, Terminator, TextRange, Ty,
};

use crate::convert::{checked_store, coerce, convert, default_value, to_string_from};
use crate::number::{JavaRandom, number_format, parse_java_double};
use crate::strings::{compare_to, to_rust};
use crate::value::*;

/// Maximum number of active calls (`init` and `entry` included). Deeper
/// recursion raises `java.lang.StackOverflowError`. The JVM's default
/// 1 MB thread stack holds roughly this many small B4J frames.
pub const MAX_DEPTH: usize = 10_000;

/// Largest array a program may allocate, in elements. Smaller ones can
/// still fail to allocate: both raise `OutOfMemoryError`.
const MAX_ARRAY: usize = 1 << 28;

/// Roughly how many bytes a default element allocates for itself: shared
/// values (numbers, Null, Strings) count nothing.
fn own_heap_size(v: &Value) -> usize {
    // An `Rc` adds two counters; the allocator adds its own overhead.
    let rc = 2 * std::mem::size_of::<usize>() + 16;
    match v {
        Value::List(_) | Value::Map(_) | Value::Sb(_) => {
            rc + std::mem::size_of::<RefCell<Option<Rc<()>>>>()
        }
        Value::Record(r) => {
            rc + std::mem::size_of::<RecordObj>()
                + r.fields.borrow().len() * std::mem::size_of::<Value>()
        }
        Value::Array(_) => {
            2 * rc + std::mem::size_of::<ArrayObj>() + std::mem::size_of::<RefCell<Vec<Value>>>()
        }
        _ => 0,
    }
}

/// Why execution stopped early.
pub enum Stop {
    /// An uncaught error, and the statement that raised it.
    Error(Exc, TextRange),
    /// `ExitApplication`: end the program with this exit code.
    Exit(i32),
}

/// What interrupts a statement.
pub enum Signal {
    Exc(Exc),
    Exit(i32),
}

impl From<Exc> for Signal {
    fn from(e: Exc) -> Signal {
        Signal::Exc(e)
    }
}

/// A statement either completes or asks the machine to call a function.
pub enum Step {
    Next,
    Call {
        func: FuncId,
        args: Vec<Value>,
        dest: Option<LocalId>,
    },
}

struct Frame {
    func: FuncId,
    locals: Vec<Value>,
    block: usize,
    /// Index of the next statement of `block`.
    stmt: usize,
    /// Where the caller wants the result.
    ret_to: Option<LocalId>,
}

/// Buffered stdout (flushed when large, before writing to stderr, at the
/// end of the run and if the run unwinds) and unbuffered stderr.
pub struct Output<'w> {
    stdout: &'w mut dyn Write,
    stderr: &'w mut dyn Write,
    buf: Vec<u8>,
}

/// Output is written in pieces of about this many bytes.
const CHUNK: usize = 1 << 16;

impl Output<'_> {
    /// Writes a line, as UTF-8 (an unpaired surrogate becomes `?`, as in
    /// [`to_rust`]), encoding it piece by piece: a huge String needs no
    /// second copy.
    pub fn log(&mut self, text: &[u16]) {
        let mut wrote = false;
        encode_line(text, &mut self.buf, |buf| {
            if buf.len() >= CHUNK {
                let _ = self.stdout.write_all(buf);
                buf.clear();
                wrote = true;
            }
        });
        // A long line is written whole, its line feed included.
        if wrote || self.buf.len() >= CHUNK {
            self.flush();
        }
    }

    pub fn log_error(&mut self, text: &[u16]) {
        self.flush();
        let mut buf = Vec::new();
        encode_line(text, &mut buf, |buf| {
            if buf.len() >= CHUNK {
                let _ = self.stderr.write_all(buf);
                buf.clear();
            }
        });
        let _ = self.stderr.write_all(&buf);
        let _ = self.stderr.flush();
    }

    pub fn flush(&mut self) {
        // Output errors (a closed pipe) are ignored, as Java's PrintStream does.
        let _ = self.stdout.write_all(&self.buf);
        let _ = self.stdout.flush();
        self.buf.clear();
    }
}

/// Appends `text` and a line feed to `buf` as UTF-8, calling `drain` as
/// it grows.
fn encode_line(text: &[u16], buf: &mut Vec<u8>, mut drain: impl FnMut(&mut Vec<u8>)) {
    for c in char::decode_utf16(text.iter().copied()) {
        let mut utf8 = [0; 4];
        buf.extend_from_slice(c.unwrap_or('?').encode_utf8(&mut utf8).as_bytes());
        drain(buf);
    }
    buf.push(b'\n');
}

impl Drop for Output<'_> {
    fn drop(&mut self) {
        self.flush();
    }
}

pub struct Machine<'p, 'w> {
    pub program: &'p Program,
    globals: Vec<Value>,
    frames: Vec<Frame>,
    /// The error caught by the innermost `Catch` that ran.
    pub last_exception: Option<Rc<Exc>>,
    pub random: Option<JavaRandom>,
    pub out: Output<'w>,
    /// String constants, converted to UTF-16 once (keyed by address).
    strings: HashMap<usize, Str>,
}

impl<'p, 'w> Machine<'p, 'w> {
    pub fn new(
        program: &'p Program,
        stdout: &'w mut dyn Write,
        stderr: &'w mut dyn Write,
    ) -> Machine<'p, 'w> {
        Machine {
            program,
            globals: program
                .globals
                .iter()
                .map(|g| default_value(&g.ty, program))
                .collect(),
            frames: Vec::new(),
            last_exception: None,
            random: None,
            out: Output {
                stdout,
                stderr,
                buf: Vec::new(),
            },
            strings: HashMap::new(),
        }
    }

    /// Runs `func` with `args` to completion.
    pub fn call(&mut self, func: FuncId, args: Vec<Value>) -> Result<(), Stop> {
        self.push_frame(func, args, None);
        self.execute()
    }

    fn push_frame(&mut self, func: FuncId, args: Vec<Value>, ret_to: Option<LocalId>) {
        let f = self.program.function(func);
        let mut locals: Vec<Value> = Vec::with_capacity(f.locals.len());
        let mut args = args.into_iter();
        for (i, local) in f.locals.iter().enumerate() {
            let arg = if (i as u32) < f.params {
                args.next()
            } else {
                None
            };
            locals.push(match arg {
                Some(a) => coerce(&local.ty, a),
                None => default_value(&local.ty, self.program),
            });
        }
        self.frames.push(Frame {
            func,
            locals,
            block: 0,
            stmt: 0,
            ret_to,
        });
    }

    fn execute(&mut self) -> Result<(), Stop> {
        let program = self.program;
        while let Some(frame) = self.frames.last_mut() {
            let function = program.function(frame.func);
            let Some(block) = function.blocks.get(frame.block) else {
                let e = Exc::internal(format!("{}: missing block {}", function.name, frame.block));
                self.raise(e, TextRange::default())?;
                continue;
            };
            if let Some(stmt) = block.stmts.get(frame.stmt) {
                frame.stmt += 1;
                let mut locals = std::mem::take(&mut frame.locals);
                let result = self.statement(&stmt.kind, &mut locals, function);
                if let Some(frame) = self.frames.last_mut() {
                    frame.locals = locals;
                }
                match result {
                    Ok(Step::Next) => {}
                    Ok(Step::Call { func, args, dest }) => {
                        if self.frames.len() >= MAX_DEPTH {
                            self.raise(Exc::bare(STACK_OVERFLOW), stmt.range)?;
                        } else {
                            self.push_frame(func, args, dest);
                        }
                    }
                    Err(Signal::Exc(e)) => self.raise(e, stmt.range)?,
                    Err(Signal::Exit(code)) => return Err(Stop::Exit(code)),
                }
                continue;
            }
            match &block.terminator {
                Terminator::Goto(b) => {
                    frame.block = b.0 as usize;
                    frame.stmt = 0;
                }
                Terminator::Branch {
                    cond,
                    then,
                    otherwise,
                } => match self.operand_in_frame(cond) {
                    Value::Bool(c) => {
                        let frame = self.frames.last_mut().ok_or(Stop::Exit(0))?;
                        frame.block = if c { then.0 } else { otherwise.0 } as usize;
                        frame.stmt = 0;
                    }
                    _ => self.raise(
                        Exc::internal("branch condition is not a Boolean"),
                        TextRange::default(),
                    )?,
                },
                Terminator::Return(value) => {
                    let value = match value {
                        Some(op) => self.operand_in_frame(op),
                        None => default_value(&function.ret, program),
                    };
                    let done = self.frames.pop();
                    if let Some(Frame {
                        ret_to: Some(dest), ..
                    }) = done
                        && let Some(caller) = self.frames.last_mut()
                    {
                        let ty = &program.function(caller.func).locals[dest.0 as usize].ty;
                        caller.locals[dest.0 as usize] = coerce(ty, value);
                    }
                }
            }
        }
        Ok(())
    }

    fn operand_in_frame(&mut self, op: &Operand) -> Value {
        match op {
            Operand::Local(l) => self
                .frames
                .last()
                .and_then(|f| f.locals.get(l.0 as usize))
                .cloned()
                .unwrap_or(Value::Null),
            Operand::Const(c) => self.constant(c),
        }
    }

    /// Unwinds to the nearest handler. `StackOverflowError` is a Java
    /// `Error`, which B4J's `catch (Exception e)` does not catch.
    fn raise(&mut self, e: Exc, range: TextRange) -> Result<(), Stop> {
        let catchable = e.kind != STACK_OVERFLOW && e.kind != OUT_OF_MEMORY;
        while let Some(frame) = self.frames.last_mut() {
            let function = self.program.function(frame.func);
            if catchable && let Some(h) = function.blocks.get(frame.block).and_then(|b| b.handler) {
                self.last_exception = Some(Rc::new(e));
                frame.block = h.0 as usize;
                frame.stmt = 0;
                return Ok(());
            }
            self.frames.pop();
        }
        Err(Stop::Error(e, range))
    }

    fn constant(&mut self, c: &Constant) -> Value {
        match c {
            Constant::Bool(b) => Value::Bool(*b),
            Constant::Int(i) => Value::Int(*i),
            Constant::Long(i) => Value::Long(*i),
            Constant::Double(d) => Value::Double(*d),
            Constant::Null => Value::Null,
            Constant::String(s) => Value::Str(
                self.strings
                    .entry(s.as_ptr() as usize)
                    .or_insert_with(|| Rc::new(s.encode_utf16().collect()))
                    .clone(),
            ),
        }
    }

    pub fn operand(&mut self, locals: &[Value], op: &Operand) -> Value {
        match op {
            Operand::Local(l) => locals.get(l.0 as usize).cloned().unwrap_or(Value::Null),
            Operand::Const(c) => self.constant(c),
        }
    }

    fn operands(&mut self, locals: &[Value], ops: &[Operand]) -> Vec<Value> {
        ops.iter().map(|o| self.operand(locals, o)).collect()
    }

    fn statement(
        &mut self,
        kind: &StatementKind,
        locals: &mut [Value],
        function: &Function,
    ) -> Result<Step, Signal> {
        match kind {
            StatementKind::Assign { dest, value } => {
                if let Some(step) = self.call_step(value, locals, Some(*dest))? {
                    return Ok(step);
                }
                let v = self.rvalue(value, locals)?;
                let i = dest.0 as usize;
                let ty = &function
                    .locals
                    .get(i)
                    .ok_or_else(|| Exc::internal("missing local"))?
                    .ty;
                // A record field is copied as is: a wrapper field that was
                // never initialised is a Java `null` reference (members on it
                // raise NullPointerException), not an uninitialised wrapper.
                locals[i] = match value {
                    Rvalue::Field { .. } => v,
                    _ => coerce(ty, v),
                };
            }
            StatementKind::Eval(value) => {
                if let Some(step) = self.call_step(value, locals, None)? {
                    return Ok(step);
                }
                self.rvalue(value, locals)?;
            }
            StatementKind::SetGlobal { global, value } => {
                let v = self.operand(locals, value);
                let i = global.0 as usize;
                let ty = &self
                    .program
                    .globals
                    .get(i)
                    .ok_or_else(|| Exc::internal("missing global"))?
                    .ty;
                self.globals[i] = coerce(ty, v);
            }
            StatementKind::SetIndex {
                array,
                indices,
                value,
            } => {
                let array = self.operand(locals, array);
                let indices = self.operands(locals, indices);
                let value = self.operand(locals, value);
                let a = as_array(&array)?;
                let i = flat_index(a, &indices)?;
                let value = checked_store(&a.elem, value, self.program)?;
                a.items.borrow_mut()[i] = value;
            }
            StatementKind::SetField {
                record,
                field,
                value,
            } => {
                let record = self.operand(locals, record);
                let value = self.operand(locals, value);
                let r = self.as_record(&record)?;
                let ty = self.field_ty(r, *field)?;
                // Fields exist before `Initialize`, as in Java.
                r.fields.borrow_mut()[*field as usize] = coerce(&ty, value);
            }
        }
        Ok(Step::Next)
    }

    /// Calls that run user code become a [`Step::Call`].
    fn call_step(
        &mut self,
        value: &Rvalue,
        locals: &[Value],
        dest: Option<LocalId>,
    ) -> Result<Option<Step>, Signal> {
        Ok(match value {
            Rvalue::Call { func, args } => Some(Step::Call {
                func: *func,
                args: self.operands(locals, args),
                dest,
            }),
            Rvalue::Builtin {
                builtin: Builtin::RecordInitialize,
                receiver: Some(r),
                ..
            } => {
                let record = self.operand(locals, r);
                let rec = self.as_record(&record)?;
                let def = self
                    .program
                    .records
                    .get(rec.id.0 as usize)
                    .ok_or_else(|| Exc::internal("missing record"))?;
                // The fields already exist; `init` gives them their B4X
                // defaults.
                rec.initialized.set(true);
                Some(Step::Call {
                    func: def.init,
                    args: vec![record.clone()],
                    dest: None,
                })
            }
            _ => None,
        })
    }

    pub fn as_record<'v>(&self, v: &'v Value) -> Result<&'v Rc<RecordObj>, Exc> {
        match v {
            Value::Record(r) => Ok(r),
            Value::Null => Err(Exc::bare(NULL_POINTER)),
            _ => Err(Exc::internal("not a record")),
        }
    }

    fn field_ty(&self, r: &RecordObj, field: u32) -> Result<Ty, Exc> {
        self.program
            .records
            .get(r.id.0 as usize)
            .and_then(|rec| rec.fields.get(field as usize))
            .map(|f| f.ty.clone())
            .ok_or_else(|| Exc::internal("missing record field"))
    }

    fn rvalue(&mut self, value: &Rvalue, locals: &[Value]) -> Result<Value, Signal> {
        let program = self.program;
        Ok(match value {
            Rvalue::Use(op) => self.operand(locals, op),
            Rvalue::Global(g) => self
                .globals
                .get(g.0 as usize)
                .cloned()
                .ok_or_else(|| Exc::internal("missing global"))?,
            Rvalue::Default(ty) => default_value(ty, program),
            Rvalue::Convert { value, from, to } => {
                let v = self.operand(locals, value);
                convert(v, from, to, program)?
            }
            Rvalue::Neg { value, .. } => match self.operand(locals, value) {
                Value::Byte(x) => Value::Int(-(x as i32)),
                Value::Short(x) => Value::Int(-(x as i32)),
                Value::Int(x) => Value::Int(x.wrapping_neg()),
                Value::Long(x) => Value::Long(x.wrapping_neg()),
                Value::Float(x) => Value::Float(-x),
                Value::Double(x) => Value::Double(-x),
                _ => return Err(Exc::internal("negating a non-number").into()),
            },
            Rvalue::Arith {
                op,
                ty,
                left,
                right,
            } => {
                let (l, r) = (self.operand(locals, left), self.operand(locals, right));
                arith(*op, ty, &l, &r)?
            }
            Rvalue::Concat(parts) => {
                let mut pieces = Vec::with_capacity(parts.len());
                for p in parts {
                    match self.operand(locals, p) {
                        Value::Str(s) => pieces.push(s),
                        other => {
                            if let Value::Str(s) = to_string_from(&other, &Ty::Object, program)? {
                                pieces.push(s);
                            }
                        }
                    }
                }
                let len = pieces
                    .iter()
                    .fold(0, |n: usize, s| n.saturating_add(s.len()));
                let mut out = Vec::new();
                try_grow_text(&mut out, len)?;
                for s in pieces {
                    out.extend_from_slice(&s);
                }
                Value::from_utf16(out)
            }
            Rvalue::Compare {
                op,
                kind,
                left,
                right,
            } => {
                let (l, r) = (self.operand(locals, left), self.operand(locals, right));
                Value::Bool(compare(*op, *kind, &l, &r, program)?)
            }
            Rvalue::Not(op) => match self.operand(locals, op) {
                Value::Bool(b) => Value::Bool(!b),
                _ => return Err(Exc::internal("Not of a non-Boolean").into()),
            },
            Rvalue::Call { .. } => {
                return Err(Exc::internal("call outside a statement").into());
            }
            Rvalue::Builtin {
                builtin,
                receiver,
                args,
            } => {
                let receiver = receiver.as_ref().map(|r| self.operand(locals, r));
                let args = self.operands(locals, args);
                self.builtin(*builtin, receiver, args)?
            }
            Rvalue::Index { array, indices } => {
                let array = self.operand(locals, array);
                let indices = self.operands(locals, indices);
                let a = as_array(&array)?;
                let i = flat_index(a, &indices)?;
                a.items.borrow()[i].clone()
            }
            Rvalue::Field { record, field } => {
                let record = self.operand(locals, record);
                let r = self.as_record(&record)?;
                r.fields
                    .borrow()
                    .get(*field as usize)
                    .cloned()
                    .ok_or_else(|| Exc::internal("missing record field"))?
            }
            Rvalue::NewArray { elem, dims } => {
                let mut sizes = Vec::with_capacity(dims.len());
                let mut total: usize = 1;
                for d in dims {
                    let n = int_of(&self.operand(locals, d))?;
                    if n < 0 {
                        return Err(Exc::new(NEGATIVE_ARRAY_SIZE, n.to_string()).into());
                    }
                    total = total.saturating_mul(n as usize);
                    sizes.push(n as usize);
                }
                if total > MAX_ARRAY {
                    return Err(out_of_memory().into());
                }
                if sizes.is_empty() {
                    sizes.push(0);
                    total = 0;
                }
                let mut items = Vec::new();
                try_grow(&mut items, total)?;
                let first = default_value(elem, program);
                match own_heap_size(&first) {
                    // A number, Null or the (immutable) empty String: every
                    // element shares it.
                    0 => items.resize(total, first),
                    // A wrapper, record or array per element. Their small
                    // allocations cannot fail gracefully, so check first that
                    // they fit.
                    size if total > 0 => {
                        check_room(total.saturating_mul(size))?;
                        items.push(first);
                        items.extend((1..total).map(|_| default_value(elem, program)));
                    }
                    _ => {}
                }
                Value::Array(Rc::new(ArrayObj {
                    elem: elem.clone(),
                    dims: sizes,
                    items: Rc::new(RefCell::new(items)),
                }))
            }
            Rvalue::ArrayLiteral { elem, items } => {
                let items: Vec<Value> = self
                    .operands(locals, items)
                    .into_iter()
                    .map(|v| coerce(elem, v))
                    .collect();
                Value::Array(Rc::new(ArrayObj {
                    elem: elem.clone(),
                    dims: vec![items.len()],
                    items: Rc::new(RefCell::new(items)),
                }))
            }
            Rvalue::CreateMap(pairs) => {
                let mut map = OrderedMap::default();
                for (k, v) in pairs {
                    let k = self.operand(locals, k);
                    let v = self.operand(locals, v);
                    map.put(k, v)?;
                }
                Value::new_map(map)
            }
            Rvalue::Format { format, value } => {
                let v = self.operand(locals, value);
                Value::str(&smart_format(format, &v, program)?)
            }
            Rvalue::IterSize(c) => {
                let c = self.operand(locals, c);
                Value::Int(iter_size(&c)? as i32)
            }
            Rvalue::IterGet { collection, index } => {
                let c = self.operand(locals, collection);
                let i = int_of(&self.operand(locals, index))?;
                iter_get(&c, i)?
            }
        })
    }
}

pub fn int_of(v: &Value) -> Result<i32, Exc> {
    match *v {
        Value::Int(i) => Ok(i),
        Value::Byte(i) => Ok(i as i32),
        Value::Short(i) => Ok(i as i32),
        Value::Char(i) => Ok(i as i32),
        Value::Long(i) => Ok(i as i32),
        _ => Err(Exc::internal("expected an Int")),
    }
}

pub fn as_array(v: &Value) -> Result<&Rc<ArrayObj>, Exc> {
    match v {
        Value::Array(a) => Ok(a),
        Value::Null => Err(Exc::bare(NULL_POINTER)),
        _ => Err(Exc::internal("not an array")),
    }
}

/// Row-major position of `indices`, each checked against its dimension.
fn flat_index(a: &ArrayObj, indices: &[Value]) -> Result<usize, Exc> {
    if indices.len() != a.dims.len() {
        return Err(Exc::internal("wrong number of array indices"));
    }
    let mut flat = 0usize;
    for (idx, &dim) in indices.iter().zip(&a.dims) {
        let i = int_of(idx)?;
        if i < 0 || i as usize >= dim {
            return Err(Exc::index(ARRAY_INDEX_OUT_OF_BOUNDS, i as i64, dim));
        }
        flat = flat * dim + i as usize;
    }
    Ok(flat)
}

fn arith_zero() -> Exc {
    Exc::new(ARITHMETIC, "/ by zero")
}

/// Java arithmetic in `ty`. Byte and Short compute as int, then narrow.
pub fn arith(op: ArithOp, ty: &Ty, l: &Value, r: &Value) -> Result<Value, Exc> {
    let int = |v: &Value| {
        v.integer_i64()
            .ok_or_else(|| Exc::internal("expected an integer"))
    };
    let float = |v: &Value| {
        v.number_f64()
            .ok_or_else(|| Exc::internal("expected a number"))
    };
    match ty {
        Ty::Byte | Ty::Short | Ty::Int => {
            let (a, b) = (int(l)? as i32, int(r)? as i32);
            let v = match op {
                ArithOp::Add => a.wrapping_add(b),
                ArithOp::Sub => a.wrapping_sub(b),
                ArithOp::Mul => a.wrapping_mul(b),
                ArithOp::Div if b == 0 => return Err(arith_zero()),
                ArithOp::Div => a.wrapping_div(b),
                ArithOp::Mod if b == 0 => return Err(arith_zero()),
                ArithOp::Mod => a.wrapping_rem(b),
            };
            Ok(match ty {
                Ty::Byte => Value::Byte(v as i8),
                Ty::Short => Value::Short(v as i16),
                _ => Value::Int(v),
            })
        }
        Ty::Long => {
            let (a, b) = (int(l)?, int(r)?);
            Ok(Value::Long(match op {
                ArithOp::Add => a.wrapping_add(b),
                ArithOp::Sub => a.wrapping_sub(b),
                ArithOp::Mul => a.wrapping_mul(b),
                ArithOp::Div if b == 0 => return Err(arith_zero()),
                ArithOp::Div => a.wrapping_div(b),
                ArithOp::Mod if b == 0 => return Err(arith_zero()),
                ArithOp::Mod => a.wrapping_rem(b),
            }))
        }
        Ty::Float => {
            let (a, b) = (float(l)? as f32, float(r)? as f32);
            Ok(Value::Float(match op {
                ArithOp::Add => a + b,
                ArithOp::Sub => a - b,
                ArithOp::Mul => a * b,
                ArithOp::Div => a / b,
                ArithOp::Mod => a % b,
            }))
        }
        _ => {
            let (a, b) = (float(l)?, float(r)?);
            Ok(Value::Double(match op {
                ArithOp::Add => a + b,
                ArithOp::Sub => a - b,
                ArithOp::Mul => a * b,
                ArithOp::Div => a / b,
                ArithOp::Mod => a % b,
            }))
        }
    }
}

fn ordered(op: CmpOp, c: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering::*;
    match op {
        CmpOp::Eq => c == Equal,
        CmpOp::Ne => c != Equal,
        CmpOp::Lt => c == Less,
        CmpOp::Gt => c == Greater,
        CmpOp::Le => c != Greater,
        CmpOp::Ge => c != Less,
    }
}

pub fn compare(
    op: CmpOp,
    kind: CmpKind,
    l: &Value,
    r: &Value,
    program: &Program,
) -> Result<bool, Exc> {
    let num = |v: &Value| {
        v.number_f64()
            .or_else(|| v.integer_i64().map(|i| i as f64))
            .ok_or_else(|| Exc::internal("expected a number"))
    };
    match kind {
        CmpKind::Numeric => {
            let (a, b) = (num(l)?, num(r)?);
            Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a < b,
                CmpOp::Gt => a > b,
                CmpOp::Le => a <= b,
                CmpOp::Ge => a >= b,
            })
        }
        CmpKind::String => match (l, r) {
            (Value::Str(a), Value::Str(b)) => Ok(ordered(op, a.as_ref().cmp(b.as_ref()))),
            _ if matches!(op, CmpOp::Eq | CmpOp::Ne) => {
                let same = matches!((l, r), (Value::Null, Value::Null));
                Ok(same == (op == CmpOp::Eq))
            }
            _ => Err(Exc::bare(NULL_POINTER)),
        },
        CmpKind::Boolean => match (l, r) {
            (Value::Bool(a), Value::Bool(b)) => Ok(ordered(op, a.cmp(b))),
            _ => Err(Exc::internal("expected Booleans")),
        },
        CmpKind::Char => {
            let (a, b) = (num(l)?, num(r)?);
            Ok(ordered(op, a.total_cmp(&b)))
        }
        CmpKind::Object => match op {
            CmpOp::Eq => java_equals(l, r),
            CmpOp::Ne => Ok(!java_equals(l, r)?),
            _ => match (l, r) {
                (Value::Str(a), Value::Str(b)) => Ok(ordered(op, compare_to(a, b).cmp(&0))),
                _ if l.is_number() && r.is_number() => compare(op, CmpKind::Numeric, l, r, program),
                _ => Err(Exc::new(
                    CLASS_CAST,
                    format!(
                        "class {} cannot be cast to class java.lang.Comparable",
                        java_class_name(l, program).unwrap_or_else(|| "null".into())
                    ),
                )),
            },
        },
    }
}

/// `Common.SmartStringFormatter` for the formats that reach the MIR:
/// `xml`, and `N` / `N.M` (`NumberFormat(value, N, M)`, unlimited fraction
/// digits without `.M`; `NaN` for a value that is not a number).
fn smart_format(format: &str, v: &Value, program: &Program) -> Result<String, Exc> {
    if format == "xml" {
        let text = match to_string_from(v, &Ty::Object, program)? {
            Value::Str(s) => to_rust(&s),
            _ => "null".into(),
        };
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            match c {
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                c => out.push(c),
            }
        }
        return Ok(out);
    }
    let bad = || Exc::new(NUMBER_FORMAT, format!("For input string: \"{format}\""));
    let (min_int, max_frac) = match format.split_once('.') {
        Some((i, f)) => (
            i.parse::<i32>().map_err(|_| bad())?,
            f.parse::<i32>().map_err(|_| bad())?,
        ),
        None => (format.parse::<i32>().map_err(|_| bad())?, i32::MAX),
    };
    let x = match v {
        Value::Str(s) => parse_java_double(&to_rust(s)).unwrap_or(f64::NAN),
        other => other.number_f64().unwrap_or(f64::NAN),
    };
    Ok(number_format(x, min_int, max_frac, 0, true))
}

fn list_inner(w: &Wrapper<ListRef>) -> Result<ListRef, Exc> {
    w.borrow()
        .clone()
        .ok_or_else(|| Exc::not_initialized("List"))
}

fn iter_size(c: &Value) -> Result<usize, Exc> {
    Ok(match c {
        Value::Array(a) => a.items.borrow().len(),
        Value::List(w) => list_inner(w)?.len()?,
        Value::View(m, _) => m.borrow().len(),
        Value::Null => return Err(Exc::bare(NULL_POINTER)),
        _ => return Err(Exc::internal("not iterable")),
    })
}

fn iter_get(c: &Value, i: i32) -> Result<Value, Exc> {
    let oob = |len| Exc::index(INDEX_OUT_OF_BOUNDS, i as i64, len);
    if i < 0 {
        return Err(oob(iter_size(c)?));
    }
    let i = i as usize;
    match c {
        Value::Array(a) => {
            let items = a.items.borrow();
            items
                .get(i)
                .cloned()
                .ok_or_else(|| Exc::index(ARRAY_INDEX_OUT_OF_BOUNDS, i as i64, items.len()))
        }
        Value::List(w) => {
            let l = list_inner(w)?;
            l.get(i)?.ok_or_else(|| oob(l.len().unwrap_or(0)))
        }
        Value::View(m, keys) => {
            let len = m.borrow().len();
            let (k, v) = m.borrow_mut().entry_at(i).ok_or_else(|| oob(len))?;
            Ok(if *keys { k } else { v })
        }
        Value::Null => Err(Exc::bare(NULL_POINTER)),
        _ => Err(Exc::internal("not iterable")),
    }
}
