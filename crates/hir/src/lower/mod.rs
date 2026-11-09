//! CST → HIR: name resolution, type checking and conversion insertion.
//!
//! Runs on a tree without syntax errors. Rules follow `docs/spec/level0.md`;
//! points marked open there are implemented as their provisional rule.

use std::collections::{HashMap, HashSet};

use ferro_syntax::{NodeOrToken, SyntaxKind, SyntaxKind::*, SyntaxNode, SyntaxToken, TextRange};

use crate::Diagnostic;
use crate::builtins::{self, Owner, Signature};
use crate::hir::*;
use crate::ty::{RecordId, Ty, builtin_type, can_convert};

mod consts;
mod expr;
mod items;
mod stmt;
mod subs;

/// Lowers a parsed module.
pub fn lower(root: &SyntaxNode) -> (Module, Vec<Diagnostic>) {
    let (module, diags, _) = lower_session(root, &SessionBindings::default());
    (module, diags)
}

/// Opaque checked session bindings. Cloning stages a submission atomically.
#[derive(Debug, Clone, Default)]
pub struct SessionBindings {
    globals: Vec<(Var, bool)>,
}

pub fn lower_session(
    root: &SyntaxNode,
    bindings: &SessionBindings,
) -> (Module, Vec<Diagnostic>, SessionBindings) {
    let mut lowerer = Lowerer::default();
    for (var, loop_declared) in &bindings.globals {
        let origin = if *loop_declared {
            Origin::Counter
        } else if var.is_const {
            Origin::Const
        } else {
            Origin::Dim
        };
        lowerer.declare_global(var.name.clone(), var.ty.clone(), var.range, origin);
        let id = *lowerer.globals.get(&key(&var.name)).unwrap();
        lowerer.module.globals[id.0 as usize].is_const = var.is_const;
        if var.is_const {
            lowerer.const_states.insert(id, ConstState::Single);
        }
    }
    lowerer.module_items(root);
    let globals = lowerer
        .module
        .globals
        .iter()
        .map(|var| (var.clone(), lowerer.declared_by_loop(&key(&var.name))))
        .collect();
    let Lowerer {
        module, mut diags, ..
    } = lowerer;
    diags.sort_by(|a, b| (a.range.start(), &a.message).cmp(&(b.range.start(), &b.message)));
    diags.dedup();
    (module, diags, SessionBindings { globals })
}

#[derive(Default)]
struct Lowerer {
    module: Module,
    diags: Vec<Diagnostic>,
    records: HashMap<String, RecordId>,
    subs: HashMap<String, SubId>,
    globals: HashMap<String, GlobalId>,
    /// Loop variables of `Process_Globals` already refused with an error.
    refused_loop_names: HashSet<String>,
    /// What declared each module variable first.
    global_origins: HashMap<String, Origin>,
    /// The loops of the Sub being lowered (`Process_Globals` included) over
    /// each name with a Sub's name, in source order, up to the current one.
    sub_named_loops: HashMap<String, LoopsBefore>,
    /// The loop variables refused where they are declared, by the range of
    /// the loop's name: a typed `For` with a Sub's name, or a typed loop of
    /// another type than its variable's.
    refused_loop_vars: HashSet<TextRange>,
    /// What `declare_process_globals` found about the constants of
    /// `Process_Globals`.
    const_states: HashMap<GlobalId, ConstState>,
    /// Whether to lower each declaration again of a constant's name in
    /// `Process_Globals`, as `declare_process_globals` decided, by the range
    /// of its declarator.
    const_declarations: HashMap<TextRange, bool>,
    /// How the current Sub declares each local name, in source order
    /// (`hoist_locals`).
    local_history: HashMap<String, LocalHistory>,
    // State of the Sub being lowered.
    locals: Vec<Var>,
    local_names: HashMap<String, LocalId>,
    /// In `Process_Globals` or `Class_Globals`.
    in_process_globals: bool,
    in_class_globals: bool,
    /// How many of `locals` are the Sub's parameters.
    params: usize,
    /// `Class_Globals`: rejected, but its variables are still declared so a
    /// class gets one diagnostic instead of one per use of its fields.
    class_globals: Option<SubId>,
    /// `ARRAY_BOUNDS` of record fields, lowered after globals and Subs exist.
    pending_bounds: Vec<(RecordId, usize, SyntaxNode)>,
    ret: Ty,
    loops: u32,
    /// Loops and Selects: what `Exit` can leave.
    exits: u32,
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

// CST helpers.

fn tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
}

fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    tokens(node).any(|t| t.kind() == kind)
}

fn child(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    node.children().find(|n| n.kind() == kind)
}

/// Text of a `NAME` / `NAME_REF` node.
fn name_of(node: &SyntaxNode) -> String {
    tokens(node)
        .next()
        .map(|t| t.text().to_string())
        .unwrap_or_default()
}

fn is_expr(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        LITERAL
            | NAME_REF
            | ME_EXPR
            | PAREN_EXPR
            | PREFIX_EXPR
            | BIN_EXPR
            | IS_EXPR
            | CALL_EXPR
            | MEMBER_EXPR
            | SMART_STRING
            | ARRAY_EXPR
            | CREATE_MAP_EXPR
    )
}

fn is_stmt(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        DIM_STMT
            | TYPE_DECL
            | IF_STMT
            | SELECT_STMT
            | FOR_STMT
            | FOR_EACH_STMT
            | DO_STMT
            | TRY_STMT
            | RETURN_STMT
            | EXIT_STMT
            | CONTINUE_STMT
            | ASSIGN_STMT
            | EXPR_STMT
            | WAIT_FOR_STMT
            | DIRECTIVE_LINE
            | INLINE_JAVA_BLOCK
    )
}

fn exprs(node: &SyntaxNode) -> Vec<SyntaxNode> {
    node.children().filter(|n| is_expr(n.kind())).collect()
}

fn range(node: &SyntaxNode) -> TextRange {
    // Without the leading/trailing trivia the parser may attach. Search from
    // each end and stop at the first real token: walking every token of the
    // node made deep trees quadratic, since this runs for every node.
    let full = node.text_range();
    let inside = |t: &SyntaxToken| full.contains_range(t.text_range());
    let mut first = node.first_token();
    while let Some(t) = first.take_if(|t| t.kind().is_trivia()) {
        first = t.next_token().filter(inside);
    }
    let mut last = node.last_token();
    while let Some(t) = last.take_if(|t| t.kind().is_trivia()) {
        last = t.prev_token().filter(inside);
    }
    match (first, last) {
        (Some(f), Some(l)) => TextRange::new(f.text_range().start(), l.text_range().end()),
        _ => full,
    }
}

/// The loop variable of a `For ... As` in `Process_Globals`, or of any
/// loop in `Class_Globals`: declaring it in the loop there is open, so it
/// must be declared with `Dim`.
fn undeclared_loop_variable(name: &str, statement: &str, globals: &str) -> String {
    format!(
        "the `{statement}` variable `{name}` must be declared first in `{globals}`, \
         e.g. `Dim {name} As Int`: declaring it in the loop is not supported there yet"
    )
}

/// A loop that declares, with `As`, a variable already declared in
/// `Process_Globals` or `Class_Globals`, where `As` in the loop is open.
fn typed_loop_variable(name: &str, statement: &str, globals: &str) -> String {
    format!(
        "declaring the `{statement}` variable `{name}` in the loop is not supported in \
         `{globals}` yet: `{name}` is already declared, so write the loop without `As`"
    )
}

/// A declarator of `Process_Globals`, as `declare_process_globals` sees it.
struct GlobalDeclaration {
    range: TextRange,
    /// Its name's range.
    at: TextRange,
    ty: Ty,
    is_const: bool,
    /// Its value is a literal, as recorded.
    literal: bool,
    /// A statement of `Process_Globals` itself, not inside a block.
    direct: bool,
    /// A statement of an `If` block that is a statement of
    /// `Process_Globals` itself.
    in_if: bool,
}

/// Which two declarations of `Process_Globals` declare a constant's name
/// (recorded, in each order).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pair {
    /// A `Const`, then a `Dim`: the name may then be assigned in a Sub
    /// (recorded: `const_dim_after_assign`).
    ConstDim,
    /// A `Dim`, then a `Const`: the name cannot be assigned (recorded:
    /// `const_after_dim_assign`).
    DimConst,
    /// Two `Const`s (recorded: `const_twice`).
    ConstConst,
}

/// What the declarations of `Process_Globals` make of a constant.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConstState {
    /// Declared there once, by its `Const` alone.
    Single,
    /// Declared by two declarations there, as `Pair` lists, and nothing
    /// else: built, each setting its value in turn (recorded). What else
    /// may change it is not known, asked once (`reported`).
    Redeclared { reported: bool, pair: Pair },
    /// Declared again where that is refused as open: the program has an
    /// error already, so the uses of the name are refused with no error of
    /// their own.
    Reported,
}

/// Why a program is refused: what B4J answers (recorded), or a question
/// no recording settles yet, refused until one does
/// (`docs/open-questions.md`). Every provisional refusal is worded here.
enum Refusal {
    Recorded(String),
    Open {
        /// What the name is, e.g. "`n` is a `Const`".
        subject: String,
        /// What is not known, e.g. "a loop may loop over it here".
        question: String,
        /// What to write instead, when there is a way.
        advice: Option<String>,
    },
}

impl Refusal {
    fn open(
        subject: impl Into<String>,
        question: impl Into<String>,
        advice: impl Into<String>,
    ) -> Refusal {
        Refusal::Open {
            subject: subject.into(),
            question: question.into(),
            advice: Some(advice.into()),
        }
    }

    /// A question with no advice to give.
    fn asked(subject: impl Into<String>, question: impl Into<String>) -> Refusal {
        Refusal::Open {
            subject: subject.into(),
            question: question.into(),
            advice: None,
        }
    }

    fn message(self) -> String {
        match self {
            Refusal::Recorded(message) => message,
            Refusal::Open {
                subject,
                question,
                advice,
            } => match advice {
                Some(advice) => {
                    format!("{subject}: whether {question} is not known yet, so {advice}")
                }
                None => format!("{subject}: whether {question} is not known yet"),
            },
        }
    }
}

/// A declaration again of a constant's name, or a `Const` of a name declared
/// before, of the same type: not recorded.
fn const_again_open(name: &str, is_const: bool) -> String {
    if is_const {
        Refusal::open(
            format!("`{name}` is already declared"),
            "a `Const` may declare it again here",
            "use another name",
        )
    } else {
        Refusal::open(
            format!("`{name}` is a `Const`"),
            "another declaration may declare it again here",
            "use another name",
        )
    }
    .message()
}

/// A module variable declared both by a `Const` and by a `Dim`: whether it
/// may change is not known.
fn redeclared_const(name: &str, what: &str) -> Refusal {
    Refusal::asked(
        format!("`{name}` is declared both by a `Const` and by a `Dim`"),
        format!("it {what}"),
    )
}

/// A `Dim` with a Sub's name. B4J: "Variable name cannot be the same as a
/// sub name" (recorded, in `Process_Globals` and in a Sub).
fn variable_named_like_sub(name: &str) -> String {
    format!("a variable cannot have the name of a Sub: `{name}` is a Sub")
}

/// The loops of a Sub so far over a name with a Sub's name.
#[derive(Default)]
struct LoopsBefore {
    /// A loop was refused: the program has an error, and the loops after
    /// it are not judged.
    unrecorded: bool,
}

/// Whether B4J accepts a loop over a name with a Sub's name, once the
/// variable's own rules (its type, a `For` over a number) are met. Its
/// recordings accept every `For` and `For Each v As T` over it, one after
/// the other, nested, three deep, from another Sub over a variable of
/// `Process_Globals`, and of any type the variable has; they refuse every
/// `For Each` without `As` ("Current declaration does not match previous
/// one"), first or not (recording runs 12 to 14: the
/// `*_named_like_sub*` tests).
fn recorded_loop(origin: Origin) -> bool {
    origin != Origin::UntypedForEach
}

/// Which declarations of one name disagree on its type, for
/// `Lowerer::type_clash`. B4J refuses each ("Current declaration does not
/// match previous one", recorded).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clash {
    /// Two declarations of a local of the current Sub.
    InSub,
    /// Two declarations of a module variable.
    Global,
    /// A `Dim` of `Process_Globals` after a loop there declared the name.
    DimAfterLoop,
    /// A loop of `Process_Globals` after the name was declared there.
    LoopInProcessGlobals,
    /// A local of a Sub with a module variable's name.
    LocalOfModuleVariable,
    /// A `For Each` without `As`, which declares an `Object` (recorded:
    /// `for_each_untyped_declared_after`), over a variable of another type.
    UntypedForEach,
}

/// A statement of a body that declares, assigns or uses a name, as
/// `in_source_order` finds it.
enum Found {
    /// A `Dim` or `Const`: each declarator declares its name.
    Dim(SyntaxNode),
    /// A loop over `name`.
    Loop {
        name: SyntaxNode,
        origin: Origin,
        type_ref: Option<SyntaxNode>,
    },
    /// The target of `name = …`.
    Assign(SyntaxNode),
    /// A use of a name.
    Use(SyntaxNode),
}

/// The one place that finds what a body (a Sub's, `Process_Globals`')
/// declares, assigns and, with `uses`, uses, in source order: a statement
/// before the names in it.
fn in_source_order(body: &SyntaxNode, uses: bool) -> impl Iterator<Item = Found> {
    body.descendants()
        .filter_map(move |node| match node.kind() {
            DIM_STMT => Some(Found::Dim(node)),
            FOR_STMT | FOR_EACH_STMT => {
                let name = child(&node, NAME)?;
                let type_ref = child(&node, TYPE_REF);
                let origin = Origin::of_loop(node.kind(), type_ref.is_some());
                Some(Found::Loop {
                    name,
                    origin,
                    type_ref,
                })
            }
            ASSIGN_STMT if uses => exprs(&node)
                .into_iter()
                .next()
                .filter(|target| target.kind() == NAME_REF)
                .map(Found::Assign),
            NAME_REF if uses => Some(Found::Use(node)),
            _ => None,
        })
}

/// What declares a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Param,
    Dim,
    Const,
    /// `For n = …`, which declares an undeclared `n` an `Int`.
    Counter,
    /// `For n As T = …`.
    TypedCounter,
    /// `For Each n As T In …`.
    TypedForEach,
    /// `For Each n In …`, which declares an undeclared `n` an `Object`
    /// (recorded: `for_each_untyped_declared_after`).
    UntypedForEach,
    /// `n = …` with `n` declared nowhere, which declares a `String`
    /// (recorded: `undeclared_variable_type`).
    Assignment,
}

impl Origin {
    /// A declarator of a `Dim` or a `Const`.
    fn of_declarator(decl: &SyntaxNode) -> Origin {
        if in_const(decl) {
            Origin::Const
        } else {
            Origin::Dim
        }
    }

    /// A `For` or `For Each` statement, by its kind and whether it has `As`.
    fn of_loop(kind: SyntaxKind, typed: bool) -> Origin {
        match (kind, typed) {
            (FOR_STMT, false) => Origin::Counter,
            (FOR_STMT, true) => Origin::TypedCounter,
            (_, true) => Origin::TypedForEach,
            (_, false) => Origin::UntypedForEach,
        }
    }

    fn is_loop(self) -> bool {
        matches!(
            self,
            Origin::Counter | Origin::TypedCounter | Origin::TypedForEach | Origin::UntypedForEach
        )
    }
}

/// How the current Sub declares a local name, in source order.
#[derive(Debug, Clone, Copy)]
struct LocalHistory {
    /// The declaration that made it a local of this Sub.
    first: Origin,
    /// A `Dim` or `Const` declares it, first or later.
    dimmed: bool,
    /// Where the name is first used, when that is before its first
    /// declaration.
    used_before: Option<TextRange>,
    /// The name is assigned before its first declaration.
    assigned_before: bool,
}

impl LocalHistory {
    /// A parameter: declared before the body, so before anything in it.
    fn param() -> Self {
        LocalHistory {
            first: Origin::Param,
            dimmed: false,
            used_before: None,
            assigned_before: false,
        }
    }

    fn new(first: Origin, before: &SoFar, k: &str) -> Self {
        LocalHistory {
            first,
            dimmed: matches!(first, Origin::Dim | Origin::Const),
            used_before: before.used.get(k).copied(),
            assigned_before: before.assigned.contains(k),
        }
    }
}

/// The names a Sub uses and assigns, so far in source order.
#[derive(Default)]
struct SoFar {
    /// Each name used, at its first use.
    used: HashMap<String, TextRange>,
    assigned: HashSet<String>,
}

/// An operand that is itself a comparison, not in parentheses.
fn is_comparison(node: &SyntaxNode) -> bool {
    node.kind() == BIN_EXPR
        && tokens(node)
            .next()
            .is_some_and(|t| matches!(t.kind(), EQ | NEQ | LT | GT | LT_EQ | GT_EQ))
}

/// What a loop declares its variable as, as `in_source_order` says.
fn loop_origin(node: &SyntaxNode) -> Origin {
    Origin::of_loop(node.kind(), child(node, TYPE_REF).is_some())
}

/// Whether a declarator belongs to a `Const` statement.
fn in_const(decl: &SyntaxNode) -> bool {
    decl.parent()
        .is_some_and(|dim| child(&dim, CONST_MODIFIER).is_some())
}

/// Why an assignment's target cannot be assigned, once reported.
enum Unassignable {
    /// A constant, or part of one, of this type: its value is still checked.
    Const(Ty),
    Reported,
}

/// What a `For` / `For Each` loops over.
enum LoopVar {
    Found(Option<(VarRef, Ty)>),
    /// Reported, and the loop checked for its own errors.
    Refused,
}

fn error_expr(range: TextRange) -> Expr {
    Expr {
        kind: ExprKind::Error,
        ty: Ty::Error,
        range,
    }
}
