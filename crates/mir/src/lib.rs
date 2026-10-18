//! The MIR: a typed, desugared intermediate representation of a Level 0
//! program, shared by the interpreter and, later, every backend.
//!
//! This module is the **contract** between the HIR → MIR lowering
//! ([`lower`](crate::lower)) and the consumers of the MIR (`ferro-interp`).
//! Semantics of conversions, operators and built-ins are those of
//! `docs/spec/level0.md` and `docs/spec/level0-builtins.md`; this file only
//! says how they are encoded.
//!
//! Shape:
//!
//! - A [`Program`] has records, globals and functions. `init` runs once
//!   (`Process_Globals`), then `entry` (`AppStart`) with the command line.
//! - A [`Function`] is a graph of [`BasicBlock`]s over typed locals.
//!   Block 0 is the entry. Locals `0..params` are the parameters; the others
//!   are the Sub's variables and compiler temporaries.
//! - Every block ends with a [`Terminator`]. Nothing is implicit: there is
//!   no `For Each`, no smart string, no short-circuit operator, no `Select`,
//!   no `Exit`/`Continue`; implicit conversions are [`Rvalue::Convert`].
//! - Exceptions: when a statement of a block raises a runtime error and the
//!   block has a `handler`, the error becomes the current `LastException`
//!   and control jumps to the handler block. Without a handler the error
//!   propagates to the caller (and, out of `entry`, stops the program).
//!   `Try ... Catch` sets the handler of every block of the Try body.

pub use ferro_hir::builtins::{Builtin, Owner, Signature};
pub use ferro_hir::hir::{ArithOp, CmpKind, CmpOp};
pub use ferro_hir::ty::{RecordId, Ty};
pub use ferro_syntax::TextRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub u32);

/// Index into [`Function::locals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

/// Index into [`Function::blocks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone)]
pub struct Program {
    pub records: Vec<Record>,
    pub globals: Vec<Global>,
    pub functions: Vec<Function>,
    /// `Process_Globals` (an empty function if the module has none). Before
    /// it runs, every global holds `Rvalue::Default` of its type.
    pub init: FuncId,
    /// `AppStart`: one parameter, the arguments as `String()`.
    pub entry: FuncId,
}

impl Program {
    pub fn function(&self, id: FuncId) -> &Function {
        &self.functions[id.0 as usize]
    }
}

/// A `Type` declaration.
#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub fields: Vec<RecordField>,
    /// A synthetic function with one parameter, the record, that gives every
    /// field its initial value (defaults, and new arrays for fields declared
    /// with bounds). `Builtin::RecordInitialize` allocates the record's field
    /// storage, then calls it.
    pub init: FuncId,
}

#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct Function {
    /// The Sub's name as written; synthetic functions use `Type.Initialize`.
    pub name: String,
    pub params: u32,
    pub locals: Vec<Local>,
    /// `Ty::Void` for a Sub without `As`.
    pub ret: Ty,
    /// Never empty; block 0 is the entry.
    pub blocks: Vec<BasicBlock>,
}

impl Function {
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0 as usize]
    }

    pub fn local(&self, id: LocalId) -> &Local {
        &self.locals[id.0 as usize]
    }
}

/// A variable or a compiler temporary. Every local starts as
/// `Rvalue::Default` of its type each time the function is called.
#[derive(Debug, Clone)]
pub struct Local {
    /// `None` for temporaries.
    pub name: Option<String>,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub terminator: Terminator,
    /// Where control goes when a statement of this block raises.
    pub handler: Option<BlockId>,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    /// Source range, for "Error occurred on line: N" messages.
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    /// `dest = value`. The value's type is `dest`'s type.
    Assign {
        dest: LocalId,
        value: Rvalue,
    },
    SetGlobal {
        global: GlobalId,
        value: Operand,
    },
    /// `array(i, j) = value`. Raises `java.lang.ArrayIndexOutOfBoundsException`.
    SetIndex {
        array: Operand,
        indices: Vec<Operand>,
        value: Operand,
    },
    /// `record.field = value`. Raises if the record is not initialised.
    SetField {
        record: Operand,
        field: u32,
        value: Operand,
    },
    /// An expression evaluated for its effect: a call whose result, if any,
    /// is discarded.
    Eval(Rvalue),
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Goto(BlockId),
    /// `cond` is a Boolean.
    Branch {
        cond: Operand,
        then: BlockId,
        otherwise: BlockId,
    },
    /// `None` returns `Rvalue::Default` of the function's return type
    /// (or nothing for `Ty::Void`).
    Return(Option<Operand>),
}

#[derive(Debug, Clone)]
pub enum Operand {
    Local(LocalId),
    Const(Constant),
}

/// Typed constants. A `Null` constant has the reference type it is used as.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    String(String),
    Null,
}

#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(Operand),
    Global(GlobalId),
    /// The default value of a type: 0, `False`, `""`, `Chr(0)`, `Null` for
    /// Object, an empty array of rank 1 for array types, and a fresh
    /// **uninitialised** object for `List`, `Map`, `StringBuilder` and records.
    /// `Dim x As T` without initialiser assigns this each time it runs.
    Default(Ty),
    /// The implicit or explicit conversions of `docs/spec/level0.md`.
    /// Raises e.g. `java.lang.NumberFormatException`.
    Convert {
        value: Operand,
        from: Ty,
        to: Ty,
    },
    /// Unary minus; the operand already has the result type.
    Neg {
        value: Operand,
        ty: Ty,
    },
    /// Both operands already have type `ty` (`Div` is always Double).
    /// Integer `Mod` by zero raises `java.lang.ArithmeticException`.
    Arith {
        op: ArithOp,
        ty: Ty,
        left: Operand,
        right: Operand,
    },
    /// String concatenation of all parts, which are Strings.
    Concat(Vec<Operand>),
    /// A Boolean. Operands are already converted for `kind` (Double for
    /// `Numeric`, String, Boolean, Char; `Object` compares boxed values with
    /// Java `equals`, and `Null` by reference).
    Compare {
        op: CmpOp,
        kind: CmpKind,
        left: Operand,
        right: Operand,
    },
    /// Logical not of a Boolean (the `Not` keyword; also used by lowering).
    Not(Operand),
    Call {
        func: FuncId,
        args: Vec<Operand>,
    },
    /// A built-in from `ferro_hir::builtins` (`Log`, `List.Add`, ...), with
    /// arguments already converted to the signature's parameter types.
    /// `RecordInitialize` calls the record's `init` function.
    Builtin {
        builtin: Builtin,
        receiver: Option<Operand>,
        args: Vec<Operand>,
    },
    /// `array(i, j)`.
    Index {
        array: Operand,
        indices: Vec<Operand>,
    },
    /// `record.field`.
    Field {
        record: Operand,
        field: u32,
    },
    /// `Dim a(3, 4) As T`: a new array, every element `Default(elem)`.
    NewArray {
        elem: Ty,
        dims: Vec<Operand>,
    },
    /// `Array As T(a, b)`: a new rank-1 array.
    ArrayLiteral {
        elem: Ty,
        items: Vec<Operand>,
    },
    /// `CreateMap(k: v, ...)`: a new initialised Map, pairs inserted in order.
    CreateMap(Vec<(Operand, Operand)>),
    /// A smart-string placeholder with a formatter (`$1.2{x}`, `$xml{s}`):
    /// `Common.SmartStringFormatter(format, value)`. `format` is lower-case
    /// and non-empty; `value` is an Object for number formats, a String for
    /// `xml`.
    Format {
        format: String,
        value: Operand,
    },
    /// Number of elements of an array (rank 1), a `List`, or a `Map.Keys` /
    /// `Map.Values` view. `For Each` reads it once, before the loop.
    IterSize(Operand),
    /// Element `index` (an Int) of the same collections, as `For Each`
    /// visits them; an Object for List and Map views. For a Map view the
    /// elements are visited in insertion order.
    IterGet {
        collection: Operand,
        index: Operand,
    },
}

impl Rvalue {
    /// Whether evaluating this can run user code or have side effects
    /// (calls and built-ins); everything else only computes a value.
    pub fn has_effects(&self) -> bool {
        matches!(self, Rvalue::Call { .. } | Rvalue::Builtin { .. })
    }
}

mod lower;
mod print;
mod validate;

pub use lower::lower;
pub use print::{dump, dump_function};
pub use validate::validate;

#[cfg(test)]
mod tests;
