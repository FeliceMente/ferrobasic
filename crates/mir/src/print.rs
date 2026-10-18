//! A readable text rendering of a [`Program`], for tests and debugging.
//!
//! ```text
//! fn AppStart(_0: String()) -> nothing
//!   let _1 i: Int
//!   let _2: Boolean
//! bb0:
//!   _1 = 1
//!   goto bb1
//! bb1 (catch bb4):
//!   ...
//! ```
//!
//! Locals print as `_N`, globals as `gN`, functions as `fN`. The format is
//! not stable and not meant to be parsed.

use std::fmt::{self, Write};

use crate::*;

/// Renders the whole program.
pub fn dump(program: &Program) -> String {
    let mut out = String::new();
    for (i, g) in program.globals.iter().enumerate() {
        let _ = writeln!(out, "global g{i} {}: {}", g.name, g.ty);
    }
    for r in &program.records {
        let fields: Vec<String> = r
            .fields
            .iter()
            .map(|f| format!("{}: {}", f.name, f.ty))
            .collect();
        let _ = writeln!(
            out,
            "type {}({}) init f{}",
            r.name,
            fields.join(", "),
            r.init.0
        );
    }
    let _ = writeln!(out, "init f{} entry f{}", program.init.0, program.entry.0);
    for (i, f) in program.functions.iter().enumerate() {
        let _ = write!(out, "\n// f{i}\n{}", dump_function(f));
    }
    out
}

/// Renders one function.
pub fn dump_function(f: &Function) -> String {
    let mut out = String::new();
    let params: Vec<String> = f.locals[..f.params as usize]
        .iter()
        .enumerate()
        .map(|(i, l)| format!("_{i}: {}", l.ty))
        .collect();
    let _ = writeln!(out, "fn {}({}) -> {}", f.name, params.join(", "), f.ret);
    for (i, l) in f.locals.iter().enumerate().skip(f.params as usize) {
        match &l.name {
            Some(n) => _ = writeln!(out, "  let _{i} {n}: {}", l.ty),
            None => _ = writeln!(out, "  let _{i}: {}", l.ty),
        }
    }
    for (i, b) in f.blocks.iter().enumerate() {
        match b.handler {
            Some(h) => _ = writeln!(out, "bb{i} (catch bb{}):", h.0),
            None => _ = writeln!(out, "bb{i}:"),
        }
        for s in &b.stmts {
            let _ = writeln!(out, "  {}", Stmt(&s.kind));
        }
        let _ = writeln!(out, "  {}", Term(&b.terminator));
    }
    out
}

struct Stmt<'a>(&'a StatementKind);
struct Term<'a>(&'a Terminator);
struct Op<'a>(&'a Operand);
struct Rv<'a>(&'a Rvalue);
struct Ops<'a>(&'a [Operand]);

impl fmt::Display for Stmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            StatementKind::Assign { dest, value } => write!(f, "_{} = {}", dest.0, Rv(value)),
            StatementKind::SetGlobal { global, value } => {
                write!(f, "g{} = {}", global.0, Op(value))
            }
            StatementKind::SetIndex {
                array,
                indices,
                value,
            } => write!(f, "{}[{}] = {}", Op(array), Ops(indices), Op(value)),
            StatementKind::SetField {
                record,
                field,
                value,
            } => write!(f, "{}.{field} = {}", Op(record), Op(value)),
            StatementKind::Eval(value) => write!(f, "eval {}", Rv(value)),
        }
    }
}

impl fmt::Display for Term<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Terminator::Goto(b) => write!(f, "goto bb{}", b.0),
            Terminator::Branch {
                cond,
                then,
                otherwise,
            } => write!(f, "branch {} bb{} bb{}", Op(cond), then.0, otherwise.0),
            Terminator::Return(None) => f.write_str("return"),
            Terminator::Return(Some(v)) => write!(f, "return {}", Op(v)),
        }
    }
}

impl fmt::Display for Op<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Operand::Local(l) => write!(f, "_{}", l.0),
            Operand::Const(Constant::Bool(b)) => write!(f, "{b}"),
            Operand::Const(Constant::Int(i)) => write!(f, "{i}"),
            Operand::Const(Constant::Long(i)) => write!(f, "{i}L"),
            Operand::Const(Constant::Double(d)) => write!(f, "{d:?}"),
            Operand::Const(Constant::String(s)) => write!(f, "{s:?}"),
            Operand::Const(Constant::Null) => f.write_str("null"),
        }
    }
}

impl fmt::Display for Ops<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, o) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{}", Op(o))?;
        }
        Ok(())
    }
}

impl fmt::Display for Rv<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Rvalue::Use(o) => write!(f, "{}", Op(o)),
            Rvalue::Global(g) => write!(f, "g{}", g.0),
            Rvalue::Default(ty) => write!(f, "default {ty}"),
            Rvalue::Convert { value, from, to } => {
                write!(f, "convert {from}->{to} {}", Op(value))
            }
            Rvalue::Neg { value, ty } => write!(f, "neg {ty} {}", Op(value)),
            Rvalue::Arith {
                op,
                ty,
                left,
                right,
            } => write!(f, "{op:?} {ty} {}, {}", Op(left), Op(right)),
            Rvalue::Concat(parts) => write!(f, "concat({})", Ops(parts)),
            Rvalue::Compare {
                op,
                kind,
                left,
                right,
            } => write!(f, "{op:?} {kind:?} {}, {}", Op(left), Op(right)),
            Rvalue::Not(o) => write!(f, "not {}", Op(o)),
            Rvalue::Call { func, args } => write!(f, "call f{}({})", func.0, Ops(args)),
            Rvalue::Builtin {
                builtin,
                receiver,
                args,
            } => {
                if let Some(r) = receiver {
                    write!(f, "{}.", Op(r))?;
                }
                write!(f, "{builtin:?}({})", Ops(args))
            }
            Rvalue::Index { array, indices } => write!(f, "{}[{}]", Op(array), Ops(indices)),
            Rvalue::Field { record, field } => write!(f, "{}.{field}", Op(record)),
            Rvalue::NewArray { elem, dims } => write!(f, "new {elem}[{}]", Ops(dims)),
            Rvalue::ArrayLiteral { elem, items } => write!(f, "array {elem}({})", Ops(items)),
            Rvalue::CreateMap(pairs) => {
                f.write_str("map(")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}: {}", Op(k), Op(v))?;
                }
                f.write_str(")")
            }
            Rvalue::Format { format, value } => write!(f, "format {format:?} {}", Op(value)),
            Rvalue::IterSize(o) => write!(f, "itersize {}", Op(o)),
            Rvalue::IterGet { collection, index } => {
                write!(f, "iterget {}[{}]", Op(collection), Op(index))
            }
        }
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&dump(self))
    }
}
