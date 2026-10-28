//! The typed, name-resolved tree of a Level 0 module.
//!
//! Every expression carries its type. Every implicit conversion is an
//! explicit [`ExprKind::Convert`] node: the MIR and the interpreter never
//! convert on their own.

use ferro_syntax::TextRange;

use crate::builtins::Builtin;
use crate::ty::{RecordId, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalId(pub u32);

/// Index into [`Sub::locals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub records: Vec<Record>,
    pub globals: Vec<Var>,
    pub subs: Vec<Sub>,
    /// `Process_Globals`, run once before `AppStart`.
    pub process_globals: Option<SubId>,
    /// `AppStart(Args() As String)`, the entry point.
    pub app_start: Option<SubId>,
}

impl Module {
    pub fn sub(&self, id: SubId) -> &Sub {
        &self.subs[id.0 as usize]
    }

    pub fn global(&self, id: GlobalId) -> &Var {
        &self.globals[id.0 as usize]
    }

    pub fn record(&self, id: RecordId) -> &Record {
        &self.records[id.0 as usize]
    }
}

/// A `Type Name (field As T, ...)` declaration.
#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub fields: Vec<Field>,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
    /// For `Items(10) As Int`: the array bounds, evaluated at initialisation.
    pub bounds: Vec<Expr>,
}

/// A variable: module-level global, parameter or local.
#[derive(Debug, Clone)]
pub struct Var {
    /// As first spelled in the source (names are case-insensitive).
    pub name: String,
    pub ty: Ty,
    pub range: TextRange,
    /// Declared by a `Const`: it cannot be assigned.
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct Sub {
    pub name: String,
    /// Parameters are the first `params` locals.
    pub params: u32,
    /// Parameters, then every local of the Sub, hoisted (scope is the whole Sub).
    pub locals: Vec<Var>,
    pub ret: Ty,
    pub body: Block,
    pub range: TextRange,
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarRef {
    Global(GlobalId),
    Local(LocalId),
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// One declarator of a `Dim` / `Private` / `Public` that runs: resets the
    /// variable each time it executes.
    Dim {
        var: VarRef,
        init: DimInit,
    },
    Assign {
        target: Place,
        value: Expr,
    },
    Expr(Expr),
    /// `If` / `Else If` chain; single-line Ifs use the same shape.
    If {
        branches: Vec<(Expr, Block)>,
        else_block: Option<Block>,
    },
    /// The scrutinee and every case value are converted for `kind`.
    Select {
        scrutinee: Expr,
        kind: CmpKind,
        cases: Vec<Case>,
        else_block: Option<Block>,
    },
    For {
        var: VarRef,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Block,
    },
    ForEach {
        var: VarRef,
        collection: Expr,
        /// Converts the current element (`Object` for List and Map views)
        /// to the variable's type.
        element: Ty,
        body: Block,
    },
    /// `Do While` (`until == false`) or `Do Until`.
    Do {
        cond: Expr,
        until: bool,
        body: Block,
    },
    Try {
        body: Block,
        catch: Block,
    },
    Return(Option<Expr>),
    /// Leaves the innermost loop or `Select`, whichever is closer.
    Exit,
    Continue,
}

#[derive(Debug, Clone)]
pub enum DimInit {
    /// The type's default value; a fresh, uninitialised object for List and Map.
    Default,
    Value(Expr),
    /// `Dim a(10, 2) As Int`: a new array with these bounds.
    Array(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct Case {
    pub values: Vec<Expr>,
    pub body: Block,
}

/// The target of an assignment.
#[derive(Debug, Clone)]
pub enum Place {
    Var(VarRef),
    Index { array: Expr, indices: Vec<Expr> },
    Field { record: Expr, field: u32 },
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Literal(Literal),
    Var(VarRef),
    /// Explicit conversion: every implicit B4X conversion becomes one.
    Convert(Box<Expr>),
    Neg(Box<Expr>),
    /// Arithmetic on two operands already converted to `Expr::ty`.
    Arith(ArithOp, Box<Expr>, Box<Expr>),
    /// String concatenation; both operands are Strings.
    Concat(Box<Expr>, Box<Expr>),
    Compare(CmpOp, CmpKind, Box<Expr>, Box<Expr>),
    /// Short-circuit `And` / `Or` on Booleans.
    Logic(LogicOp, Box<Expr>, Box<Expr>),
    Call {
        sub: SubId,
        args: Vec<Expr>,
    },
    Builtin {
        builtin: Builtin,
        receiver: Option<Box<Expr>>,
        args: Vec<Expr>,
    },
    Index {
        array: Box<Expr>,
        indices: Vec<Expr>,
    },
    Field {
        record: Box<Expr>,
        field: u32,
    },
    SmartString(Vec<SmartPart>),
    ArrayLiteral(Vec<Expr>),
    CreateMap(Vec<(Expr, Expr)>),
    IIf(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Produced after a diagnostic; never executed (the program is rejected).
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    String(String),
    Null,
}

#[derive(Debug, Clone)]
pub enum SmartPart {
    Text(String),
    /// A placeholder: `format` is lower-cased, empty for `${...}`.
    Value {
        expr: Expr,
        format: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// How a comparison compares; both operands are converted accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpKind {
    /// Both operands converted to Double.
    Numeric,
    /// Both Strings, by value, case-sensitive.
    String,
    Boolean,
    Char,
    /// Reference values (`x = Null`, Object `equals`).
    Object,
    /// A `Select` case, as `BA.switchObjectToInt` matches it: if the
    /// scrutinee holds a number, by `doubleValue` (a case that is not a
    /// number is a `ClassCastException`), else with the scrutinee's
    /// `equals` (a `Null` scrutinee is a `NullPointerException`).
    Switch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicOp {
    And,
    Or,
}
