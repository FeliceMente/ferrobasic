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

/// Lowers a parsed module.
pub fn lower(root: &SyntaxNode) -> (Module, Vec<Diagnostic>) {
    let mut lowerer = Lowerer::default();
    lowerer.module_items(root);
    let Lowerer {
        module, mut diags, ..
    } = lowerer;
    // Some declarations are visited by more than one pass.
    diags.sort_by(|a, b| (a.range.start(), &a.message).cmp(&(b.range.start(), &b.message)));
    diags.dedup();
    (module, diags)
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
    /// Module variables that a loop of `Process_Globals` declares.
    loop_declared: HashSet<String>,
    /// The loops of the Sub being lowered (`Process_Globals` included) over
    /// each name with a Sub's name, in source order, up to the current one.
    sub_named_loops: HashMap<String, LoopsBefore>,
    /// The loop variables refused where they are declared, by the range of
    /// the loop's name: a typed `For` with a Sub's name, or a typed loop of
    /// another type than its variable's.
    refused_loop_vars: HashSet<TextRange>,
    /// The current Sub's locals with a Sub's name: `true` for a loop
    /// variable, `false` for a refused `Dim`. Their uses mean the Sub
    /// (recorded).
    sub_named_locals: HashMap<String, bool>,
    // State of the Sub being lowered.
    locals: Vec<Var>,
    local_names: HashMap<String, LocalId>,
    /// In `Process_Globals` or `Class_Globals`.
    in_process_globals: bool,
    in_class_globals: bool,
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

/// A `Dim` with a Sub's name. B4J: "Variable name cannot be the same as a
/// sub name" (recorded, in `Process_Globals` and in a Sub).
fn variable_named_like_sub(name: &str) -> String {
    format!("a variable cannot have the name of a Sub: `{name}` is a Sub")
}

/// The loops of a Sub so far over a name with a Sub's name.
#[derive(Default, Clone)]
struct LoopsBefore {
    /// The accepted loops, in order: all over the same variable.
    seen: Vec<LoopKind>,
    /// A loop B4J's answer to is not recorded, or refused: the program has
    /// an error, and the loops after it are not judged.
    unrecorded: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LoopKind {
    Counter,
    /// `For Each v As T`.
    Typed,
    /// `For Each` without `As`.
    Untyped,
}

/// Where a loop over a name with a Sub's name is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopScope {
    /// In a Sub, over its own loop variable.
    Sub,
    /// In `Process_Globals`, over one of its loop variables.
    ProcessGlobals,
    /// In another Sub, over a loop variable of `Process_Globals`.
    Elsewhere,
    /// In another Sub, over its own loop variable, which has the name of a
    /// loop variable of `Process_Globals` too.
    SubOverModule,
}

/// What B4J's recordings say about a loop over a name with a Sub's name:
/// accepted (`Some(true)`), refused with "Current declaration does not
/// match previous one" (`Some(false)`), or not recorded (`None`). The
/// variable's type is `ty`; `before` holds the loops over it accepted
/// before in the same Sub, and `around` those around it, all over the
/// same variable. Only
/// these rows are recorded; anything else is open.
fn recorded_loop(
    scope: LoopScope,
    kind: LoopKind,
    ty: &Ty,
    before: &[LoopKind],
    around: &[LoopKind],
) -> Option<bool> {
    use LoopKind::*;
    use LoopScope::*;
    let int = |t: &Ty| *t == Ty::Int;
    // The first loop over the name in this Sub.
    let first = match (scope, kind) {
        // `local_counter_named_like_sub`, `process_globals_loop_named_like_sub`,
        // `process_globals_loop_named_like_sub_loop_elsewhere`,
        // `process_globals_for_each_named_like_sub_loop_elsewhere`.
        (_, Counter) if int(ty) => Some(true),
        // It declares the name, of any type: `local_for_each_named_like_sub`
        // (`Int`), `local_for_each_named_like_sub_member` (`String`), and
        // their `Process_Globals` twins.
        (Sub | ProcessGlobals, Typed) => Some(true),
        // `local_for_each_named_like_sub_over_module_variable`.
        (SubOverModule, Typed) if int(ty) => Some(true),
        // `process_globals_loop_named_like_sub_for_each_elsewhere`,
        // `process_globals_for_each_named_like_sub_for_each_elsewhere`.
        (Elsewhere, Untyped) if int(ty) => Some(false),
        _ => None,
    };
    // This loop after an earlier one over the name, one after the other.
    let after = |earlier: LoopKind| match (scope, earlier, kind) {
        // `local_typed_for_each_after_module_counter_named_like_sub`.
        (SubOverModule, Counter, Typed) if int(ty) => Some(true),
        (Elsewhere | SubOverModule, ..) => None,
        // `local_counter_named_like_sub_loop_again`,
        // `process_globals_loop_named_like_sub_loop_again`.
        (_, Counter, Counter) if int(ty) => Some(true),
        // `local_counter_after_for_each_named_like_sub`,
        // `process_globals_counter_after_for_each_named_like_sub`.
        (_, Typed, Counter) if int(ty) => Some(true),
        // `local_typed_for_each_after_counter_named_like_sub`,
        // `process_globals_counter_named_like_sub_typed_for_each_again`.
        (_, Counter, Typed) if int(ty) => Some(true),
        // `local_typed_for_each_named_like_sub_loop_again`,
        // `process_globals_for_each_named_like_sub_typed_for_each_again`.
        (_, Typed, Typed) if int(ty) => Some(true),
        // `local_for_each_after_counter_named_like_sub`,
        // `process_globals_loop_named_like_sub_for_each_again`.
        (_, Counter, Untyped) => Some(false),
        // `local_for_each_named_like_sub_loop_again`,
        // `process_globals_for_each_named_like_sub_for_each_again`.
        (_, Typed, Untyped) if int(ty) => Some(false),
        _ => None,
    };
    // This loop inside one over the name.
    let inside = |outer: LoopKind| match (scope, outer, kind) {
        // `local_typed_for_each_named_like_sub_nested`.
        (Sub, Typed, Typed) if int(ty) => Some(true),
        // `local_for_each_named_like_sub_nested`.
        (Sub, Counter, Untyped) => Some(false),
        _ => None,
    };
    // Every recording has at most two loops over the name: this one, and
    // one before it or around it. Anything longer is not recorded.
    match (before, around) {
        ([], []) => first,
        ([earlier], []) => after(*earlier),
        // The loop around it is the one before it.
        ([earlier], [outer]) if earlier == outer => inside(*outer),
        _ => None,
    }
}

fn loop_kind(node: &SyntaxNode) -> LoopKind {
    match (node.kind(), child(node, TYPE_REF).is_some()) {
        (FOR_STMT, _) => LoopKind::Counter,
        (_, true) => LoopKind::Typed,
        _ => LoopKind::Untyped,
    }
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

// Module level.
impl Lowerer {
    fn error(&mut self, range: TextRange, message: impl Into<String>) {
        self.diags.push(Diagnostic {
            range,
            message: message.into(),
        });
    }

    fn ty_name(&self, ty: &Ty) -> String {
        match ty {
            Ty::Record(id) => self.module.record(*id).name.clone(),
            Ty::Array(elem, rank) => {
                let commas = ",".repeat(usize::from(rank.saturating_sub(1)));
                format!("{}({commas})", self.ty_name(elem))
            }
            other => other.to_string(),
        }
    }

    /// The rank of an array with `dims` dimensions: at least 1, at most 255
    /// (the JVM's limit, so B4J cannot accept more either).
    fn rank(&mut self, dims: usize, at: TextRange) -> u8 {
        match u8::try_from(dims.max(1)) {
            Ok(rank) => rank,
            Err(_) => {
                self.error(at, "an array can have at most 255 dimensions");
                1
            }
        }
    }

    fn module_items(&mut self, root: &SyntaxNode) {
        for node in root.children() {
            match node.kind() {
                DIRECTIVE_LINE => self.unsupported_directive(&node),
                INLINE_JAVA_BLOCK => self.inline_java(&node),
                _ => {}
            }
        }
        let subs: Vec<SyntaxNode> = root.children().filter(|n| n.kind() == SUB_DECL).collect();
        self.declare_records(root);
        for sub in &subs {
            self.declare_sub(sub);
        }
        // The main module's loops declare module variables too;
        // `Class_Globals` was not recorded.
        if let Some(id) = self.module.process_globals {
            let node = subs[id.0 as usize].clone();
            self.declare_process_globals(&node);
        }
        if let Some(id) = self.class_globals {
            let node = subs[id.0 as usize].clone();
            self.declare_globals(&node);
        }
        for (id, field, node) in std::mem::take(&mut self.pending_bounds) {
            let bounds = exprs(&node)
                .iter()
                .map(|e| {
                    let e = self.expr(e);
                    self.coerce(e, &Ty::Int)
                })
                .collect();
            self.module.records[id.0 as usize].fields[field].bounds = bounds;
        }
        for (i, node) in subs.iter().enumerate() {
            self.lower_sub(SubId(i as u32), node);
        }
        let at = TextRange::empty(root.text_range().start());
        // The original toolchain refuses a main module without it, even
        // when the program has no module variables. A class has
        // `Class_Globals` instead, and gets its own Level 1 error.
        if self.module.process_globals.is_none() && self.class_globals.is_none() {
            self.error(
                at,
                "missing `Sub Process_Globals`: the main module needs one, even an empty one",
            );
        }
        if self.module.app_start.is_none() {
            self.error(
                at,
                "missing `Sub AppStart (Args() As String)`: it is the program's entry point",
            );
        }
    }

    /// One error, at the block's `#If Java`, without a trailing comment or
    /// spaces.
    fn inline_java(&mut self, node: &SyntaxNode) {
        let start = range(node).start();
        let text = node.text().to_string();
        let first = text.trim_start().split(['\r', '\n']).next().unwrap_or("");
        let first = first.split('\'').next().unwrap_or("").trim_end();
        self.error(
            TextRange::at(start, (first.len() as u32).into()),
            "inline Java (`#If Java`) is not supported at Level 0: it needs Java interop (Level 3)",
        );
    }

    fn unsupported_directive(&mut self, node: &SyntaxNode) {
        let text = node.text().to_string();
        let word = text
            .trim()
            .trim_start_matches('#')
            .split_whitespace()
            .next()
            .unwrap_or("");
        let what = if word.eq_ignore_ascii_case("if")
            || word.eq_ignore_ascii_case("else")
            || word.eq_ignore_ascii_case("end")
        {
            "conditional compilation and inline code (`#If`) are not supported at Level 0"
        } else {
            "this directive is not supported at Level 0"
        };
        self.error(range(node), format!("{what}: `{}`", text.trim()));
    }

    /// `Type` declarations, wherever they appear (normally `Process_Globals`).
    fn declare_records(&mut self, root: &SyntaxNode) {
        let decls: Vec<SyntaxNode> = root
            .descendants()
            .filter(|n| n.kind() == TYPE_DECL)
            .collect();
        for decl in &decls {
            let name = child(decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
            if self.records.contains_key(&key(&name)) || builtin_type(&name).is_some() {
                self.error(range(decl), format!("type `{name}` is already defined"));
                continue;
            }
            let id = RecordId(self.module.records.len() as u32);
            self.records.insert(key(&name), id);
            self.module.records.push(Record {
                name,
                fields: Vec::new(),
                range: range(decl),
            });
        }
        // Fields after all names, so records can refer to each other.
        for decl in &decls {
            let name = child(decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
            let Some(&id) = self.records.get(&key(&name)) else {
                continue;
            };
            if !self.module.record(id).fields.is_empty() {
                continue;
            }
            let mut fields: Vec<Field> = Vec::new();
            for field in decl.children().filter(|n| n.kind() == FIELD) {
                let field_name = child(&field, NAME).map(|n| name_of(&n)).unwrap_or_default();
                let bounds_node = child(&field, ARRAY_BOUNDS);
                let elem = match child(&field, TYPE_REF) {
                    Some(t) => self.resolve_type(&t),
                    None => Ty::String,
                };
                let ty = match &bounds_node {
                    Some(b) => {
                        let rank = self.rank(exprs(b).len(), range(b));
                        Ty::Array(Box::new(elem), rank)
                    }
                    None => elem,
                };
                // Bounds may use globals and Subs: lowered once those are declared.
                if let Some(b) = bounds_node {
                    self.pending_bounds.push((id, fields.len(), b));
                }
                if fields
                    .iter()
                    .any(|f| f.name.eq_ignore_ascii_case(&field_name))
                {
                    self.error(
                        range(&field),
                        format!("field `{field_name}` is declared twice"),
                    );
                }
                fields.push(Field {
                    name: field_name,
                    ty,
                    bounds: Vec::new(),
                });
            }
            self.module.records[id.0 as usize].fields = fields;
        }
    }

    fn resolve_type(&mut self, type_ref: &SyntaxNode) -> Ty {
        let parts: Vec<SyntaxToken> = tokens(type_ref).collect();
        let name: String = parts
            .iter()
            .take_while(|t| t.kind() != L_PAREN)
            .map(|t| t.text())
            .collect();
        let is_array = parts.iter().any(|t| t.kind() == L_PAREN);
        let base = if let Some(ty) = builtin_type(&name) {
            ty
        } else if let Some(&id) = self.records.get(&key(&name)) {
            Ty::Record(id)
        } else {
            self.error(
                range(type_ref),
                format!("unknown type `{name}`: library types are not supported at Level 0"),
            );
            Ty::Error
        };
        if is_array && base != Ty::Error {
            Ty::Array(Box::new(base), 1)
        } else {
            base
        }
    }

    fn declare_sub(&mut self, node: &SyntaxNode) {
        let name = child(node, NAME).map(|n| name_of(&n)).unwrap_or_default();
        let id = SubId(self.module.subs.len() as u32);
        let mut locals = Vec::new();
        if let Some(list) = child(node, PARAM_LIST) {
            for param in list.children().filter(|n| n.kind() == PARAM) {
                let pname = child(&param, NAME).map(|n| name_of(&n)).unwrap_or_default();
                let is_array = has_token(&param, L_PAREN);
                let mut ty = match child(&param, TYPE_REF) {
                    Some(t) => self.resolve_type(&t),
                    None => Ty::String,
                };
                if is_array && !matches!(ty, Ty::Array(..) | Ty::Error) {
                    ty = Ty::Array(Box::new(ty), 1);
                }
                if locals
                    .iter()
                    .any(|v: &Var| v.name.eq_ignore_ascii_case(&pname))
                {
                    self.error(
                        range(&param),
                        format!("parameter `{pname}` is declared twice"),
                    );
                }
                locals.push(Var {
                    name: pname,
                    ty,
                    range: range(&param),
                    is_const: false,
                });
            }
        }
        let ret = match child(node, TYPE_REF) {
            Some(t) => self.resolve_type(&t),
            None => Ty::Void,
        };
        let header = child(node, NAME)
            .map(|n| range(&n))
            .unwrap_or_else(|| range(node));
        if let std::collections::hash_map::Entry::Vacant(slot) = self.subs.entry(key(&name)) {
            slot.insert(id);
        } else {
            self.error(header, format!("Sub `{name}` is defined twice"));
        }
        let lower = key(&name);
        if lower == "class_globals" {
            self.error(
                header,
                "classes (`Class_Globals`) are not supported at Level 0: they arrive at Level 1",
            );
            self.class_globals.get_or_insert(id);
        } else if lower == "globals" || lower == "activity_create" {
            self.error(
                header,
                format!(
                    "`{name}` belongs to an Activity module, which is not supported at Level 0"
                ),
            );
        } else if lower == "process_globals" {
            if !locals.is_empty() || ret != Ty::Void {
                self.error(
                    header,
                    "`Process_Globals` takes no parameters and returns nothing",
                );
            }
            self.module.process_globals.get_or_insert(id);
        } else if lower == "appstart" {
            let ok = locals.len() == 1 && locals[0].ty == Ty::Array(Box::new(Ty::String), 1);
            if !ok {
                self.error(
                    header,
                    "the entry point must be `Sub AppStart (Args() As String)`; UI apps (with a Form) are not supported at Level 0",
                );
            }
            self.module.app_start.get_or_insert(id);
        }
        self.module.subs.push(Sub {
            name,
            params: locals.len() as u32,
            locals,
            ret,
            body: Vec::new(),
            range: range(node),
        });
    }

    /// Module variables: the declarators of `Class_Globals`.
    fn declare_globals(&mut self, sub: &SyntaxNode) {
        for dim in sub.descendants().filter(|n| n.kind() == DIM_STMT) {
            for (decl, ty) in self.declarators(&dim) {
                let name = child(&decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
                if self.subs.contains_key(&key(&name)) {
                    // Recorded for `Process_Globals` and a Sub, not here.
                    self.error(
                        range(&decl),
                        format!(
                            "`{name}` is a Sub: whether a variable of `Class_Globals` may have \
                             its name is not known yet, so use another name"
                        ),
                    );
                    self.declare_global(name, Ty::Error, range(&decl));
                    continue;
                }
                self.declare_global(name.clone(), ty, range(&decl));
                self.mark_const(&decl, &name);
            }
        }
    }

    fn declare_global(&mut self, name: String, ty: Ty, at: TextRange) {
        match self.globals.get(&key(&name)) {
            Some(&id) => {
                let existing = &self.module.global(id).ty;
                if *existing != ty && *existing != Ty::Error && ty != Ty::Error {
                    self.error(
                        at,
                        format!("`{name}` is already declared with another type"),
                    );
                }
            }
            None => {
                let id = GlobalId(self.module.globals.len() as u32);
                self.globals.insert(key(&name), id);
                self.module.globals.push(Var {
                    name,
                    ty,
                    range: at,
                    is_const: false,
                });
            }
        }
    }

    /// The module variables of `Process_Globals`, in source order: its
    /// `Dim`s, and its loops. An undeclared `For` counter is a module
    /// variable, an Int, and a `For Each v As T` variable one of type `T`,
    /// which every Sub sees and whose undeclared counters share (recorded).
    /// Every declaration of a name there must agree on its type: B4J refuses
    /// a later one that does not ("Current declaration does not match
    /// previous one").
    fn declare_process_globals(&mut self, sub: &SyntaxNode) {
        let nodes = sub
            .descendants()
            .filter(|n| matches!(n.kind(), DIM_STMT | FOR_STMT | FOR_EACH_STMT));
        for node in nodes {
            if node.kind() == DIM_STMT {
                for (decl, ty) in self.declarators(&node) {
                    let name = child(&decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
                    self.declare_dim_global(name.clone(), ty, range(&decl));
                    self.mark_const(&decl, &name);
                }
            } else {
                self.declare_loop_global(&node);
            }
        }
    }

    /// A `Dim` of `Process_Globals`. After a loop that declared the name, the
    /// same type is that variable, as after a `Dim`; another type is refused.
    fn declare_dim_global(&mut self, name: String, ty: Ty, at: TextRange) {
        let k = key(&name);
        if self.subs.contains_key(&k) {
            // B4J: "Variable name cannot be the same as a sub name" (recorded).
            self.error(at, variable_named_like_sub(&name));
            return;
        }
        if let Some(&id) = self.globals.get(&k)
            && self.loop_declared.contains(&k)
        {
            let existing = self.module.global(id).ty.clone();
            if existing != ty && existing != Ty::Error && ty != Ty::Error {
                let (existing, ty) = (self.ty_name(&existing), self.ty_name(&ty));
                self.error(
                    at,
                    format!(
                        "`{name}` is already declared as {existing} by a loop of \
                         `Process_Globals`: this `Dim` cannot declare it again as {ty}"
                    ),
                );
            }
            return;
        }
        self.declare_global(name, ty, at);
    }

    fn declare_loop_global(&mut self, node: &SyntaxNode) {
        let Some(name) = child(node, NAME) else {
            return;
        };
        let text = name_of(&name);
        let k = key(&text);
        let ty = match (node.kind(), child(node, TYPE_REF)) {
            (FOR_STMT, None) => Ty::Int,
            (FOR_EACH_STMT, Some(t)) => self.resolve_type(&t),
            // `For i As T` there is open (refused when lowered), and a
            // `For Each` without `As` uses a declared variable.
            _ => return,
        };
        if self.refused_loop_names.contains(&k) {
            return;
        }
        if let Some(&id) = self.globals.get(&k) {
            // A counter over a `Dim`'s variable of any type is that variable
            // (its type is checked when lowered). Otherwise the same type is
            // that variable, whichever declarations and in which order
            // (recorded), and another type is refused (recorded).
            let existing = self.module.global(id).ty.clone();
            let counter_over_dim = node.kind() == FOR_STMT && !self.loop_declared.contains(&k);
            if counter_over_dim || existing == ty || existing == Ty::Error || ty == Ty::Error {
                return;
            }
            let (existing, ty) = (self.ty_name(&existing), self.ty_name(&ty));
            self.refused_loop_names.insert(k);
            self.error(
                range(&name),
                format!(
                    "`{text}` is already declared as {existing} in `Process_Globals`: this \
                     loop cannot declare it again as {ty}"
                ),
            );
            return;
        }
        if self.loop_name_free(&text, &name) {
            self.loop_declared.insert(k);
            self.declare_global(text, ty, range(&name));
        }
    }

    /// Whether a loop variable that `Process_Globals` declares may take this
    /// name. A Sub's may (recorded: `lookup_value`); whether the
    /// module variable hides a
    /// `Type`, `Bit` or a keyword of the same name is not recorded, so it is
    /// refused, once.
    fn loop_name_free(&mut self, text: &str, name: &SyntaxNode) -> bool {
        let what = if self.records.contains_key(&key(text)) {
            "a `Type`"
        } else if text.eq_ignore_ascii_case("Bit") {
            "the `Bit` object"
        } else if builtins::lookup(Owner::Global, text).is_some() {
            "a keyword"
        } else {
            return true;
        };
        self.refused_loop_names.insert(key(text));
        self.error(
            range(name),
            format!(
                "the loop variable `{text}` of `Process_Globals` has the name of {what}: \
                 whether it would hide it is not known yet, so use another name"
            ),
        );
        false
    }

    /// A loop variable `declare_loop_globals` already reported.
    fn refused(&self, name: &str) -> bool {
        self.in_process_globals
            && !self.in_class_globals
            && self.refused_loop_names.contains(&key(name))
    }

    fn globals_sub(&self) -> &'static str {
        if self.in_class_globals {
            "Class_Globals"
        } else {
            "Process_Globals"
        }
    }

    /// The declarators of a `Dim` with their types: a declarator without
    /// `As` takes the type of the next `As` to its right; with none, String.
    fn declarators(&mut self, dim: &SyntaxNode) -> Vec<(SyntaxNode, Ty)> {
        let decls: Vec<SyntaxNode> = dim.children().filter(|n| n.kind() == DECLARATOR).collect();
        let mut types: Vec<Option<Ty>> = decls
            .iter()
            .map(|d| child(d, TYPE_REF).map(|t| self.resolve_type(&t)))
            .collect();
        let mut next = Ty::String;
        for ty in types.iter_mut().rev() {
            match ty {
                Some(t) => next = t.clone(),
                None => *ty = Some(next.clone()),
            }
        }
        decls
            .into_iter()
            .zip(types)
            .map(|(decl, elem)| {
                let elem = elem.unwrap();
                let ty = match child(&decl, ARRAY_BOUNDS) {
                    Some(bounds) if elem != Ty::Error => {
                        let rank = self.rank(exprs(&bounds).len(), range(&bounds));
                        match elem {
                            // `Dim a() As String()`: already an array type.
                            Ty::Array(..) => elem,
                            _ => Ty::Array(Box::new(elem), rank),
                        }
                    }
                    _ => elem,
                };
                (decl, ty)
            })
            .collect()
    }
}

// Subs and statements.
impl Lowerer {
    fn lower_sub(&mut self, id: SubId, node: &SyntaxNode) {
        let sub = &self.module.subs[id.0 as usize];
        self.locals = sub.locals.clone();
        self.local_names = self
            .locals
            .iter()
            .enumerate()
            .map(|(i, v)| (key(&v.name), LocalId(i as u32)))
            .collect();
        self.ret = sub.ret.clone();
        self.sub_named_locals.clear();
        self.sub_named_loops.clear();
        self.in_class_globals = self.class_globals == Some(id);
        self.in_process_globals = self.module.process_globals == Some(id) || self.in_class_globals;
        self.loops = 0;
        self.exits = 0;
        if !self.in_process_globals {
            // B4J: "Parameter name cannot hide global variable name" (recorded).
            let count = self.module.sub(id).params as usize;
            let clashes: Vec<(TextRange, String)> = self.locals[..count]
                .iter()
                .filter(|p| {
                    self.globals
                        .get(&key(&p.name))
                        .is_some_and(|&g| self.module.global(g).ty != Ty::Error)
                })
                .map(|p| (p.range, p.name.clone()))
                .collect();
            for (at, name) in clashes {
                self.error(
                    at,
                    format!(
                        "the parameter `{name}` has the name of a module variable: a parameter \
                         cannot hide a module variable"
                    ),
                );
            }
        }
        let Some(body) = child(node, BLOCK) else {
            return;
        };
        if !self.in_process_globals {
            self.hoist_locals(&body);
        }
        let block = self.block(&body);
        let sub = &mut self.module.subs[id.0 as usize];
        sub.body = block;
        sub.locals = std::mem::take(&mut self.locals);
    }

    fn declare_local(&mut self, name: &str, ty: Ty, at: TextRange) {
        match self.local_names.get(&key(name)) {
            Some(&id) => {
                let existing = &self.locals[id.0 as usize].ty;
                if *existing != ty && *existing != Ty::Error && ty != Ty::Error {
                    let existing = self.ty_name(existing);
                    self.refused_loop_vars.insert(at);
                    self.error(
                        at,
                        format!("`{name}` is already declared as {existing} in this Sub"),
                    );
                }
            }
            None => {
                if ty != Ty::Error
                    && !self.in_process_globals
                    && let Some(&id) = self.globals.get(&key(name))
                {
                    self.local_like_global(name, &ty, id, at);
                }
                self.push_local(name, ty, at);
            }
        }
    }

    fn push_local(&mut self, name: &str, ty: Ty, at: TextRange) {
        self.local_names
            .insert(key(name), LocalId(self.locals.len() as u32));
        self.locals.push(Var {
            name: name.to_string(),
            ty,
            range: at,
            is_const: false,
        });
    }

    /// The name a local `Dim` or `For Each v As T` declares. With a module
    /// variable's name and type it is that module variable, which a
    /// re-executed `Dim` resets (recorded). With a Sub's name a `Dim` is
    /// refused, even after a loop declared the name (recorded), and a
    /// `For Each` is a local whose uses mean the Sub (recorded).
    fn declare_dim_local(&mut self, name: &str, ty: Ty, at: TextRange, is_dim: bool) {
        let k = key(name);
        if is_dim && self.subs.contains_key(&k) {
            self.error(at, variable_named_like_sub(name));
            if !self.local_names.contains_key(&k) {
                self.push_local(name, ty, at);
            }
            // A loop of that name still means the Sub.
            self.sub_named_locals.entry(k).or_insert(false);
            return;
        }
        if !self.local_names.contains_key(&k) {
            if self.subs.contains_key(&k) {
                self.sub_named_locals.insert(k, true);
            } else if let Some(&id) = self.globals.get(&k)
                && self.module.global(id).ty == ty
            {
                return;
            }
        }
        self.declare_local(name, ty, at);
    }

    /// The counter a `For` of a Sub declares. With a Sub's name an undeclared
    /// counter is a local whose uses mean the Sub, and a typed one is refused
    /// (B4J: "Regular variable followed by '=' expected", recorded).
    fn declare_loop_local(&mut self, name: &str, ty: Ty, at: TextRange, typed: bool) {
        let k = key(name);
        let declared = self.local_names.contains_key(&k);
        if self.subs.contains_key(&k) {
            if typed {
                // Whatever declared the name first (recorded without one).
                self.refused_loop_vars.insert(at);
                self.error(
                    at,
                    format!(
                        "`{name}` is a Sub: a `For` counter with its name cannot be declared \
                         with `As`, so loop without `As`"
                    ),
                );
                if !declared {
                    self.sub_named_locals.insert(k, true);
                    self.push_local(name, ty, at);
                }
                return;
            }
            if !declared {
                self.sub_named_locals.insert(k, true);
            }
        }
        self.declare_local(name, ty, at);
    }

    /// A local declared with a module variable's name: B4J refuses another
    /// type ("Current declaration does not match previous one", recorded);
    /// a `Dim` or `For Each` of the same type is the module variable
    /// (`declare_dim_local`), and whether a `For i As T` of the same type is
    /// a new local is open, so it is refused. A `For Each` of the same type
    /// is accepted when the name is also a Sub's (recorded): its uses mean
    /// the Sub, so which variable it loops with is never observable. Another
    /// type is refused for a Sub's name too (recorded).
    fn local_like_global(&mut self, name: &str, ty: &Ty, id: GlobalId, at: TextRange) {
        let existing = self.module.global(id).ty.clone();
        if existing == Ty::Error {
            return;
        }
        let shown = self.ty_name(&existing);
        if existing == *ty && self.subs.contains_key(&key(name)) {
            return;
        }
        let message = if existing == *ty {
            format!(
                "`{name}` is also a module variable ({shown}): whether this loop declares a \
                 new local or means the module variable is not known yet, so loop over it \
                 without `As`"
            )
        } else {
            let ty = self.ty_name(ty);
            format!(
                "`{name}` is a module variable of type {shown}: a local cannot declare it \
                 again as {ty}"
            )
        };
        self.refused_loop_vars.insert(at);
        self.error(at, message);
    }

    /// Every local of a Sub is visible in the whole Sub: declare them all first.
    fn hoist_locals(&mut self, body: &SyntaxNode) {
        for node in body.descendants() {
            match node.kind() {
                DIM_STMT => {
                    for (decl, ty) in self.declarators(&node) {
                        let name = child(&decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
                        self.declare_dim_local(&name, ty, range(&decl), true);
                        self.mark_const(&decl, &name);
                    }
                }
                FOR_EACH_STMT => {
                    if let (Some(name), Some(t)) = (child(&node, NAME), child(&node, TYPE_REF)) {
                        let ty = self.resolve_type(&t);
                        self.declare_dim_local(&name_of(&name), ty, range(&name), false);
                    }
                }
                FOR_STMT => {
                    if let (Some(name), Some(t)) = (child(&node, NAME), child(&node, TYPE_REF)) {
                        let ty = self.resolve_type(&t);
                        self.declare_loop_local(&name_of(&name), ty, range(&name), true);
                    }
                }
                _ => {}
            }
        }
        // An undeclared `For` counter is implicitly an Int.
        for node in body.descendants().filter(|n| n.kind() == FOR_STMT) {
            if let Some(name) = child(&node, NAME) {
                let text = name_of(&name);
                if self.lookup_var(&text).is_none() {
                    self.declare_loop_local(&text, Ty::Int, range(&name), false);
                }
            }
        }
    }

    fn lookup_var(&self, name: &str) -> Option<(VarRef, Ty)> {
        let k = key(name);
        if let Some(&id) = self.local_names.get(&k) {
            return Some((VarRef::Local(id), self.locals[id.0 as usize].ty.clone()));
        }
        self.globals
            .get(&k)
            .map(|&id| (VarRef::Global(id), self.module.global(id).ty.clone()))
    }

    fn block(&mut self, node: &SyntaxNode) -> Block {
        let mut out = Vec::new();
        for stmt in node.children().filter(|n| is_stmt(n.kind())) {
            self.stmt(&stmt, &mut out);
        }
        out
    }

    /// A `BLOCK`, or the single statement of a single-line `If` / `Else`.
    fn body_of(&mut self, node: &SyntaxNode) -> Block {
        if let Some(block) = child(node, BLOCK) {
            return self.block(&block);
        }
        let mut out = Vec::new();
        if let Some(stmt) = node.children().find(|n| is_stmt(n.kind())) {
            self.stmt(&stmt, &mut out);
        }
        out
    }

    fn stmt(&mut self, node: &SyntaxNode, out: &mut Block) {
        let at = range(node);
        let push = |out: &mut Block, kind| out.push(Stmt { kind, range: at });
        match node.kind() {
            DIM_STMT => {
                for (decl, decl_ty) in self.declarators(node) {
                    let name = child(&decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
                    let Some((var, ty)) = self.lookup_var(&name) else {
                        continue;
                    };
                    if !self.const_declaration(&decl, &decl_ty, &var) {
                        continue;
                    }
                    let bounds = child(&decl, ARRAY_BOUNDS);
                    let init = if let Some(init) = exprs(&decl).first() {
                        // `Dim days() As String = Array As String(...)`.
                        if let Some(b) = bounds.as_ref().filter(|b| !exprs(b).is_empty()) {
                            self.error(
                                range(b),
                                "an array declared with bounds cannot also have an initial value",
                            );
                        }
                        let value = self.expr(init);
                        DimInit::Value(self.coerce(value, &ty))
                    } else if let Some(bounds) = bounds {
                        let bounds = exprs(&bounds)
                            .iter()
                            .map(|e| {
                                let e = self.expr(e);
                                self.coerce(e, &Ty::Int)
                            })
                            .collect();
                        DimInit::Array(bounds)
                    } else {
                        DimInit::Default
                    };
                    out.push(Stmt {
                        kind: StmtKind::Dim { var, init },
                        range: range(&decl),
                    });
                }
            }
            TYPE_DECL => {}
            ASSIGN_STMT => {
                let parts = exprs(node);
                let (Some(target), Some(value)) = (parts.first(), parts.get(1)) else {
                    return;
                };
                match self.place(target) {
                    Ok((place, ty)) => {
                        let value = self.expr(value);
                        let value = self.coerce(value, &ty);
                        push(
                            out,
                            StmtKind::Assign {
                                target: place,
                                value,
                            },
                        );
                    }
                    // Its errors are reported along with the constant's.
                    Err(Unassignable::Const(ty)) => {
                        let value = self.expr(value);
                        self.coerce(value, &ty);
                    }
                    Err(Unassignable::Reported) => {}
                }
            }
            EXPR_STMT => {
                let Some(e) = exprs(node).into_iter().next() else {
                    return;
                };
                let expr = self.expr(&e);
                match expr.kind {
                    ExprKind::Call { .. } | ExprKind::Builtin { .. } | ExprKind::Error => {
                        push(out, StmtKind::Expr(expr))
                    }
                    // Already reported, as its type says.
                    _ if expr.ty == Ty::Error => {}
                    _ => self.error(
                        at,
                        "this expression is not a statement: only calls can stand alone",
                    ),
                }
            }
            IF_STMT => {
                let cond = exprs(node).into_iter().next();
                let cond = self.condition(cond.as_ref(), at);
                let then = self.body_of(node);
                let mut branches = vec![(cond, then)];
                let mut else_block = None;
                for clause in node.children() {
                    match clause.kind() {
                        ELSE_IF_CLAUSE => {
                            let c = exprs(&clause).into_iter().next();
                            let c = self.condition(c.as_ref(), range(&clause));
                            let body = self.body_of(&clause);
                            branches.push((c, body));
                        }
                        ELSE_CLAUSE => else_block = Some(self.body_of(&clause)),
                        _ => {}
                    }
                }
                push(
                    out,
                    StmtKind::If {
                        branches,
                        else_block,
                    },
                );
            }
            SELECT_STMT => self.select(node, out),
            FOR_STMT => {
                let Some(name) = child(node, NAME) else {
                    return;
                };
                let text = name_of(&name);
                // `As` in `Process_Globals` is open: refuse it even when the
                // name is declared, so its type is never ignored.
                let typed_in_globals = self.in_process_globals && child(node, TYPE_REF).is_some();
                if self.refused(&text) && !typed_in_globals {
                    self.refused_loop(node, &text, &name);
                    return;
                }
                let LoopVar::Found(found) = self.loop_variable(node, &text, &name) else {
                    return;
                };
                let Some((var, ty)) = found.clone().filter(|_| !typed_in_globals) else {
                    // Every other undeclared counter is an Int (`hoist_locals`,
                    // `declare_loop_globals`): never drop the loop silently.
                    let message = if found.is_some() {
                        typed_loop_variable(&text, "For", self.globals_sub())
                    } else {
                        undeclared_loop_variable(&text, "For", self.globals_sub())
                    };
                    self.error(range(&name), message);
                    self.refused_loop(node, &text, &name);
                    return;
                };
                self.accepted_loop(node, &name, &ty);
                if !ty.is_numeric() && ty != Ty::Error {
                    let ty = self.ty_name(&ty);
                    self.error(
                        range(&name),
                        format!("a `For` counter must be a number, not {ty}"),
                    );
                }
                let parts = exprs(node);
                let bound = |this: &mut Self, i: usize| {
                    parts.get(i).map(|e| {
                        let e = this.expr(e);
                        this.coerce(e, &ty)
                    })
                };
                let (Some(start), Some(end)) = (bound(self, 0), bound(self, 1)) else {
                    return;
                };
                let step = bound(self, 2);
                let body = self.loop_body(node);
                push(
                    out,
                    StmtKind::For {
                        var,
                        start,
                        end,
                        step,
                        body,
                    },
                );
            }
            FOR_EACH_STMT => {
                let Some(name) = child(node, NAME) else {
                    return;
                };
                let text = name_of(&name);
                if self.refused(&text) {
                    self.refused_loop(node, &text, &name);
                    return;
                }
                // `Process_Globals` declares it (`declare_loop_globals`);
                // `Class_Globals` was not recorded.
                let typed_in_globals = self.in_class_globals && child(node, TYPE_REF).is_some();
                let LoopVar::Found(found) = self.loop_variable(node, &text, &name) else {
                    return;
                };
                let Some((var, ty)) = found.clone().filter(|_| !typed_in_globals) else {
                    let message = if found.is_some() {
                        typed_loop_variable(&text, "For Each", self.globals_sub())
                    } else if self.in_process_globals {
                        undeclared_loop_variable(&text, "For Each", self.globals_sub())
                    } else {
                        format!(
                            "`{text}` is not declared: write `For Each {text} As <type> In ...`"
                        )
                    };
                    self.error(range(&name), message);
                    self.refused_loop(node, &text, &name);
                    return;
                };
                self.accepted_loop(node, &name, &ty);
                let Some(coll) = exprs(node).into_iter().next() else {
                    return;
                };
                let collection = self.expr(&coll);
                let element = match &collection.ty {
                    Ty::Array(elem, 1) => (**elem).clone(),
                    Ty::List | Ty::Iterable | Ty::Object => Ty::Object,
                    Ty::Error => Ty::Error,
                    other => {
                        let other = self.ty_name(other);
                        self.error(
                            collection.range,
                            format!("`For Each` cannot iterate over {other}"),
                        );
                        Ty::Error
                    }
                };
                if !can_convert(&element, &ty) {
                    let (e, t) = (self.ty_name(&element), self.ty_name(&ty));
                    self.error(
                        range(&name),
                        format!("elements of type {e} cannot be assigned to {t}"),
                    );
                }
                let body = self.loop_body(node);
                push(
                    out,
                    StmtKind::ForEach {
                        var,
                        collection,
                        element,
                        body,
                    },
                );
            }
            DO_STMT => {
                let until = has_token(node, UNTIL_KW);
                let cond = exprs(node).into_iter().next();
                let cond = self.condition(cond.as_ref(), at);
                let body = self.loop_body(node);
                push(out, StmtKind::Do { cond, until, body });
            }
            TRY_STMT => {
                let body = child(node, BLOCK)
                    .map(|b| self.block(&b))
                    .unwrap_or_default();
                let catch = child(node, CATCH_CLAUSE)
                    .and_then(|c| child(&c, BLOCK))
                    .map(|b| self.block(&b))
                    .unwrap_or_default();
                push(out, StmtKind::Try { body, catch });
            }
            RETURN_STMT => {
                let value = exprs(node).into_iter().next().map(|e| self.expr(&e));
                let value = match value {
                    Some(v) if self.ret == Ty::Void => {
                        self.error(v.range, "this Sub has no return type (`As ...`), so `Return` cannot return a value");
                        None
                    }
                    Some(v) => {
                        let ret = self.ret.clone();
                        Some(self.coerce(v, &ret))
                    }
                    None => None,
                };
                push(out, StmtKind::Return(value));
            }
            EXIT_STMT => {
                if self.exits == 0 {
                    self.error(at, "`Exit` must be inside a loop or a `Select`");
                }
                push(out, StmtKind::Exit);
            }
            CONTINUE_STMT => {
                if self.loops == 0 {
                    self.error(at, "`Continue` must be inside a loop");
                }
                push(out, StmtKind::Continue);
            }
            DIRECTIVE_LINE => self.unsupported_directive(node),
            INLINE_JAVA_BLOCK => self.inline_java(node),
            WAIT_FOR_STMT => {
                let what = builtins::later_level("Wait").unwrap_or("resumable subs");
                self.error(
                    at,
                    format!("`Wait For` is not supported at Level 0: it needs {what}"),
                );
            }
            _ => {}
        }
    }

    /// A loop already reported, checked for its own errors: its bounds or
    /// collection, then its body. An undeclared variable is declared with
    /// the error type first, so its uses are not reported again.
    fn refused_loop(&mut self, node: &SyntaxNode, text: &str, name: &SyntaxNode) {
        let k = key(text);
        if self.subs.contains_key(&k) {
            self.sub_named_loops.entry(k).or_default().unrecorded = true;
        }
        for e in exprs(node) {
            self.expr(&e);
        }
        if self.lookup_var(text).is_none() {
            self.declare_local(text, Ty::Error, range(name));
        }
        self.loop_body(node);
    }

    /// Notes a loop lowered past its variable, when its name is a Sub's.
    fn accepted_loop(&mut self, node: &SyntaxNode, name: &SyntaxNode, ty: &Ty) {
        let k = key(&name_of(name));
        if !self.subs.contains_key(&k) {
            return;
        }
        let kind = loop_kind(node);
        let refused = self.refused_loop_already(node, name, ty);
        let seen = self.sub_named_loops.entry(k).or_default();
        if refused || kind == LoopKind::Untyped {
            seen.unrecorded = true;
        } else {
            seen.seen.push(kind);
        }
    }

    /// Whether a loop is refused elsewhere, as B4J refuses it (recorded):
    /// where its variable is declared (`refused_loop_vars`), or, for a `For`
    /// over anything but a number, when it is lowered.
    fn refused_loop_already(&self, node: &SyntaxNode, name: &SyntaxNode, ty: &Ty) -> bool {
        self.refused_loop_vars.contains(&range(name))
            || (node.kind() == FOR_STMT && !ty.is_numeric())
    }

    /// The variable a `For` / `For Each` loops over, or the loop refused
    /// (reported and checked here). Over a loop variable with a Sub's name,
    /// a loop is accepted, or refused with B4J's message, only where
    /// `recorded_loop` lists B4J's answer; anything else is refused as
    /// open. After a loop refused or not recorded, which is an error
    /// already, the loops over the name are not judged.
    fn loop_variable(&mut self, node: &SyntaxNode, text: &str, name: &SyntaxNode) -> LoopVar {
        let found = self.lookup_var(text);
        if let Some((var, _)) = &found
            && self.is_const(var)
        {
            self.error(
                range(name),
                format!(
                    "`{text}` is a `Const`: whether a loop may loop over it is not known yet, so \
                     use another name"
                ),
            );
            self.refused_loop(node, text, name);
            return LoopVar::Refused;
        }
        let k = key(text);
        if !self.subs.contains_key(&k) {
            return LoopVar::Found(found);
        }
        // A loop variable with a Sub's name: anything else was reported
        // already, or is not one.
        let (scope, ty) = match &found {
            Some((VarRef::Global(_), ty)) if self.loop_declared.contains(&k) => {
                let scope = if self.in_process_globals {
                    LoopScope::ProcessGlobals
                } else {
                    LoopScope::Elsewhere
                };
                (scope, ty.clone())
            }
            Some((VarRef::Local(_), ty)) if self.sub_named_locals.get(&k) == Some(&true) => {
                let module_ty = self
                    .globals
                    .get(&k)
                    .filter(|_| self.loop_declared.contains(&k))
                    .map(|id| self.module.global(*id).ty.clone());
                let scope = match module_ty {
                    None => LoopScope::Sub,
                    Some(module_ty) if module_ty == *ty => LoopScope::SubOverModule,
                    // Another type than the module variable's: reported
                    // already (recorded).
                    Some(_) => return LoopVar::Found(found),
                };
                (scope, ty.clone())
            }
            _ => return LoopVar::Found(found),
        };
        let kind = loop_kind(node);
        let before = self.sub_named_loops.get(&k);
        if before.is_some_and(|b| b.unrecorded) || self.refused_loop_already(node, name, &ty) {
            return LoopVar::Found(found);
        }
        // The innermost loop over the same name around this one.
        let around: Vec<LoopKind> = node
            .ancestors()
            .skip(1)
            .filter(|a| {
                matches!(a.kind(), FOR_STMT | FOR_EACH_STMT)
                    && child(a, NAME).is_some_and(|n| key(&name_of(&n)) == k)
            })
            .map(|a| loop_kind(&a))
            .collect();
        let seen = before.map_or(&[][..], |b| &b.seen);
        let answer = recorded_loop(scope, kind, &ty, seen, &around);
        if answer == Some(true) {
            return LoopVar::Found(found);
        }
        let what = if matches!(scope, LoopScope::Sub | LoopScope::SubOverModule) {
            "a loop variable of this Sub"
        } else {
            "a module variable"
        };
        let message = if answer == Some(false) {
            format!(
                "`{text}` is {what} with the name of a Sub: a `For Each` without `As` cannot \
                 loop over it again"
            )
        } else {
            let form = match kind {
                LoopKind::Counter => "this `For`",
                LoopKind::Typed => "this `For Each`",
                LoopKind::Untyped => "a `For Each` without `As`",
            };
            format!(
                "`{text}` is {what} with the name of a Sub: whether {form} may loop over it here \
                 is not known yet, so use another name"
            )
        };
        self.error(range(name), message);
        self.refused_loop(node, text, name);
        LoopVar::Refused
    }

    fn loop_body(&mut self, node: &SyntaxNode) -> Block {
        self.loops += 1;
        self.exits += 1;
        let body = child(node, BLOCK)
            .map(|b| self.block(&b))
            .unwrap_or_default();
        self.loops -= 1;
        self.exits -= 1;
        body
    }

    fn condition(&mut self, node: Option<&SyntaxNode>, at: TextRange) -> Expr {
        match node {
            Some(n) => {
                let e = self.expr(n);
                self.coerce(e, &Ty::Boolean)
            }
            None => error_expr(at),
        }
    }

    fn select(&mut self, node: &SyntaxNode, out: &mut Block) {
        let Some(scrutinee) = exprs(node).into_iter().next() else {
            return;
        };
        let scrutinee = self.expr(&scrutinee);
        // As `BA.switchObjectToInt`: numbers match by numeric value, anything
        // else with `equals` on the boxed values (so String "1" does not match
        // Int 1, and an Object holding Int 1 matches Case 1.0). A Char
        // scrutinee converts its cases to Char, as the docs'
        // `Select Txt.CharAt(i)` / `Case "A"` example needs.
        let (kind, domain) = match &scrutinee.ty {
            t if t.is_numeric() => (CmpKind::Numeric, Ty::Double),
            Ty::Char => (CmpKind::Char, Ty::Char),
            Ty::Error => (CmpKind::Switch, Ty::Error),
            _ => (CmpKind::Switch, Ty::Object),
        };
        let scrutinee = self.coerce(scrutinee, &domain);
        let mut cases = Vec::new();
        let mut else_block = None;
        self.exits += 1;
        for clause in node.children().filter(|n| n.kind() == CASE_CLAUSE) {
            let body = child(&clause, BLOCK)
                .map(|b| self.block(&b))
                .unwrap_or_default();
            if has_token(&clause, ELSE_KW) {
                else_block = Some(body);
                continue;
            }
            let values = exprs(&clause)
                .iter()
                .map(|v| {
                    let v = self.expr(v);
                    self.coerce(v, &domain)
                })
                .collect();
            cases.push(Case { values, body });
        }
        self.exits -= 1;
        out.push(Stmt {
            kind: StmtKind::Select {
                scrutinee,
                kind,
                cases,
                else_block,
            },
            range: range(node),
        });
    }

    /// The target of an assignment and its type.
    fn place(&mut self, node: &SyntaxNode) -> Result<(Place, Ty), Unassignable> {
        match node.kind() {
            NAME_REF => {
                let name = name_of(node);
                // B4J reads the name of a loop variable with a Sub's name as the
                // Sub, and refuses the `=` ("Syntax error", or "'next'
                // expected" inside its loop: recorded).
                // A `Dim` with a Sub's name was reported already: assigning it
                // is not reported again.
                let refused_dim = self.sub_named_locals.get(&key(&name)) == Some(&false);
                let found = if refused_dim {
                    self.lookup_var(&name)
                } else {
                    self.lookup_value(&name)
                };
                match found {
                    Some((var, ty)) if !refused_dim && self.is_const(&var) => {
                        // B4J: "Cannot modify Const variable." (recorded).
                        let name = self.var(&var).name.clone();
                        self.error(
                            range(node),
                            format!("`{name}` is a `Const`: it cannot be assigned"),
                        );
                        Err(Unassignable::Const(ty))
                    }
                    Some((var, ty)) => Ok((Place::Var(var), ty)),
                    None => {
                        let message = if self.subs.contains_key(&key(&name)) {
                            format!("cannot assign to `{name}`: it is a Sub, not a variable")
                        } else if builtins::lookup(Owner::Global, &name).is_some() {
                            format!(
                                "cannot assign to `{name}`: it is a built-in constant or keyword"
                            )
                        } else {
                            format!(
                                "`{name}` is not declared: declare it with `Dim {name} As <type>`"
                            )
                        };
                        self.error(range(node), message);
                        Err(Unassignable::Reported)
                    }
                }
            }
            CALL_EXPR | MEMBER_EXPR => {
                let expr = self.expr(node);
                if let Some(message) = self.const_place(&expr) {
                    self.error(range(node), message);
                    return Err(Unassignable::Const(expr.ty));
                }
                match expr.kind {
                    ExprKind::Index { array, indices } => Ok((
                        Place::Index {
                            array: *array,
                            indices,
                        },
                        expr.ty,
                    )),
                    ExprKind::Field { record, field } => Ok((
                        Place::Field {
                            record: *record,
                            field,
                        },
                        expr.ty,
                    )),
                    ExprKind::Error => Err(Unassignable::Reported),
                    _ => {
                        self.error(range(node), "cannot assign to this expression");
                        Err(Unassignable::Reported)
                    }
                }
            }
            _ => {
                self.error(range(node), "cannot assign to this expression");
                Err(Unassignable::Reported)
            }
        }
    }
}

// Constants.
impl Lowerer {
    fn var(&self, var: &VarRef) -> &Var {
        match var {
            VarRef::Global(id) => self.module.global(*id),
            VarRef::Local(id) => &self.locals[id.0 as usize],
        }
    }

    fn is_const(&self, var: &VarRef) -> bool {
        self.var(var).is_const
    }

    /// Makes the variable a declarator of a `Const` declares a constant,
    /// when this declarator is its first declaration.
    fn mark_const(&mut self, decl: &SyntaxNode, name: &str) {
        if !in_const(decl) {
            return;
        }
        let Some((var, _)) = self.lookup_var(name) else {
            return;
        };
        let declared = match var {
            VarRef::Global(id) => &mut self.module.globals[id.0 as usize],
            VarRef::Local(id) => &mut self.locals[id.0 as usize],
        };
        if declared.range == range(decl) {
            declared.is_const = true;
        }
    }

    /// Checks a declarator of a `Dim` against the constants, and whether
    /// to lower it. A `Const` must be given its value there (B4J: "Const
    /// variable must be set in the declaration line.", recorded). Whether a
    /// `Const` may declare a name declared before, or another declaration
    /// a `Const`'s, is not recorded; with another type, the clash is
    /// reported already. Nor is one of an array with bounds and no value.
    fn const_declaration(&mut self, decl: &SyntaxNode, ty: &Ty, var: &VarRef) -> bool {
        let is_const = in_const(decl);
        let declared = self.var(var);
        let name = child(decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
        let at = child(decl, NAME).map_or_else(|| range(decl), |n| range(&n));
        let same_type = declared.ty == *ty;
        if declared.range != range(decl) && same_type && (is_const || declared.is_const) {
            let message = if is_const {
                format!(
                    "`{name}` is already declared: whether a `Const` may declare it again is not \
                     known yet, so use another name"
                )
            } else {
                format!(
                    "`{name}` is a `Const`: whether another declaration may declare it again is \
                     not known yet, so use another name"
                )
            };
            self.error(at, message);
            return false;
        }
        if is_const && exprs(decl).is_empty() {
            let bounds = child(decl, ARRAY_BOUNDS).filter(|b| !exprs(b).is_empty());
            let message = if bounds.is_some() {
                format!(
                    "`{name}` is a `Const` array with bounds: whether it may have no value is not \
                     known yet, so give it its value"
                )
            } else {
                format!("`{name}` is a `Const`: give it its value where it is declared")
            };
            self.error(at, message);
            return false;
        }
        true
    }

    /// Why an item or field assignment is refused when it changes a
    /// constant: an item of a `Const` array is refused (B4J: "Cannot modify
    /// Const variable.", recorded); a field, or an item of an item, is not
    /// recorded.
    fn const_place(&self, expr: &Expr) -> Option<String> {
        let item = matches!(&expr.kind, ExprKind::Index { array, .. }
            if matches!(array.kind, ExprKind::Var(_)));
        let mut root = expr;
        loop {
            match &root.kind {
                ExprKind::Index { array, .. } => root = array,
                ExprKind::Field { record, .. } => root = record,
                ExprKind::Var(var) if self.is_const(var) => {
                    let name = &self.var(var).name;
                    return Some(if item {
                        format!("`{name}` is a `Const`: its items cannot be assigned")
                    } else {
                        format!(
                            "`{name}` is a `Const`: whether this part of it may be assigned is \
                             not known yet"
                        )
                    });
                }
                _ => return None,
            }
        }
    }
}

// Expressions.
impl Lowerer {
    /// Converts `expr` to `to`, inserting an explicit `Convert`.
    fn coerce(&mut self, expr: Expr, to: &Ty) -> Expr {
        if expr.ty == *to || expr.ty == Ty::Error || *to == Ty::Error {
            return expr;
        }
        if can_convert(&expr.ty, to) {
            let range = expr.range;
            return Expr {
                kind: ExprKind::Convert(Box::new(expr)),
                ty: to.clone(),
                range,
            };
        }
        let message = match &expr.ty {
            Ty::Void => "this call does not return a value".to_string(),
            Ty::Iterable => "`Keys` and `Values` can only be used in `For Each`".to_string(),
            Ty::Array(_, rank) if *to == Ty::List => format!(
                "an array of {rank} dimensions cannot be used as a List: only a one-dimensional array converts to one"
            ),
            from => format!(
                "cannot convert {} to {}",
                self.ty_name(from),
                self.ty_name(to)
            ),
        };
        self.error(expr.range, message);
        error_expr(expr.range)
    }

    fn expr(&mut self, node: &SyntaxNode) -> Expr {
        let at = range(node);
        match node.kind() {
            LITERAL => self.literal(node),
            NAME_REF => self.name_ref(node, None),
            PAREN_EXPR => match exprs(node).first() {
                Some(inner) => {
                    let mut e = self.expr(inner);
                    e.range = at;
                    e
                }
                None => error_expr(at),
            },
            PREFIX_EXPR => {
                let Some(inner) = exprs(node).into_iter().next() else {
                    return error_expr(at);
                };
                let inner = self.expr(&inner);
                let ty = inner.ty.promoted();
                if !can_convert(&inner.ty, &ty) {
                    return self.coerce(inner, &Ty::Double);
                }
                let inner = self.coerce(inner, &ty);
                Expr {
                    kind: ExprKind::Neg(Box::new(inner)),
                    ty,
                    range: at,
                }
            }
            BIN_EXPR => self.binary(node),
            IS_EXPR => {
                self.error(at, "type tests (`Is`) are not supported yet");
                error_expr(at)
            }
            CALL_EXPR => self.call(node),
            MEMBER_EXPR => self.member(node, None),
            SMART_STRING => self.smart_string(node),
            ARRAY_EXPR => {
                let elem = child(node, TYPE_REF)
                    .map(|t| self.resolve_type(&t))
                    .unwrap_or(Ty::Object);
                let items = child(node, ARG_LIST).map(|a| exprs(&a)).unwrap_or_default();
                let items = items
                    .iter()
                    .map(|i| {
                        let i = self.expr(i);
                        self.coerce(i, &elem)
                    })
                    .collect();
                Expr {
                    kind: ExprKind::ArrayLiteral(items),
                    ty: Ty::Array(Box::new(elem), 1),
                    range: at,
                }
            }
            CREATE_MAP_EXPR => {
                let mut pairs = Vec::new();
                for entry in node.children().filter(|n| n.kind() == MAP_ENTRY) {
                    let parts = exprs(&entry);
                    if let (Some(k), Some(v)) = (parts.first(), parts.get(1)) {
                        let k = self.expr(k);
                        let k = self.coerce(k, &Ty::Object);
                        let v = self.expr(v);
                        let v = self.coerce(v, &Ty::Object);
                        pairs.push((k, v));
                    }
                }
                Expr {
                    kind: ExprKind::CreateMap(pairs),
                    ty: Ty::Map,
                    range: at,
                }
            }
            ME_EXPR => {
                self.error(
                    at,
                    "`Me` refers to a class instance: classes arrive at Level 1",
                );
                error_expr(at)
            }
            _ => error_expr(at),
        }
    }

    fn literal(&mut self, node: &SyntaxNode) -> Expr {
        let at = range(node);
        let Some(token) = tokens(node).next() else {
            return error_expr(at);
        };
        let text = token.text();
        let (lit, ty) = match token.kind() {
            TRUE_KW => (Literal::Bool(true), Ty::Boolean),
            FALSE_KW => (Literal::Bool(false), Ty::Boolean),
            NULL_KW => (Literal::Null, Ty::Null),
            STRING => {
                let inner = text.strip_prefix('"').unwrap_or(text);
                let inner = inner.strip_suffix('"').unwrap_or(inner);
                (Literal::String(inner.to_string()), Ty::String)
            }
            REAL => match text.parse::<f64>() {
                Ok(v) => (Literal::Double(v), Ty::Double),
                Err(_) => {
                    self.error(at, "invalid number");
                    return error_expr(at);
                }
            },
            INT => {
                let parsed = match text.get(..2) {
                    Some("0x" | "0X") => u64::from_str_radix(&text[2..], 16).ok().map(|v| {
                        // Hex literals up to 32 bits are Ints: 0xFF000000 is -16777216.
                        if v <= u64::from(u32::MAX) {
                            Literal::Int(v as u32 as i32)
                        } else {
                            Literal::Long(v as i64)
                        }
                    }),
                    _ => text.parse::<i64>().ok().map(|v| match i32::try_from(v) {
                        Ok(i) => Literal::Int(i),
                        Err(_) => Literal::Long(v),
                    }),
                };
                match parsed {
                    Some(lit) => {
                        let ty = if matches!(lit, Literal::Int(_)) {
                            Ty::Int
                        } else {
                            Ty::Long
                        };
                        (lit, ty)
                    }
                    None => {
                        self.error(at, "integer literal is too large");
                        return error_expr(at);
                    }
                }
            }
            _ => return error_expr(at),
        };
        Expr {
            kind: ExprKind::Literal(lit),
            ty,
            range: at,
        }
    }

    fn sub_call(&mut self, id: SubId, args: &[SyntaxNode], full: TextRange) -> Expr {
        let sub = self.module.sub(id);
        let params: Vec<Ty> = sub.locals[..sub.params as usize]
            .iter()
            .map(|v| v.ty.clone())
            .collect();
        let ret = sub.ret.clone();
        let sub_name = sub.name.clone();
        let args = self.args(args, &params, &sub_name, full);
        Expr {
            kind: ExprKind::Call { sub: id, args },
            ty: ret,
            range: full,
        }
    }

    /// `name` looked up as a value. A loop variable with a Sub's name is
    /// `None`: a module variable a loop of `Process_Globals` declares, or a
    /// local of the current Sub. Every use of the name in
    /// an expression, bare, with arguments or as a statement, calls the Sub,
    /// and assigning it is an error (recorded).
    fn lookup_value(&self, name: &str) -> Option<(VarRef, Ty)> {
        let found = self.lookup_var(name)?;
        let k = key(name);
        let sub_named = self.subs.contains_key(&k)
            && match found.0 {
                VarRef::Global(_) => self.loop_declared.contains(&k),
                VarRef::Local(_) => self.sub_named_locals.contains_key(&k),
            };
        (!sub_named).then_some(found)
    }

    /// A bare name, or the callee of `name(args)` when `args` is given.
    fn name_ref(&mut self, node: &SyntaxNode, args: Option<&[SyntaxNode]>) -> Expr {
        let at = range(node);
        let name = name_of(node);
        let full = args.map_or(at, |_| range(&node.parent().unwrap()));
        if let Some((var, ty)) = self.lookup_value(&name) {
            let var_expr = Expr {
                kind: ExprKind::Var(var),
                ty: ty.clone(),
                range: at,
            };
            return match args {
                None => var_expr,
                Some(args) => self.index(var_expr, args, full),
            };
        }
        if let Some(&id) = self.subs.get(&key(&name)) {
            return self.sub_call(id, args.unwrap_or(&[]), full);
        }
        if name.eq_ignore_ascii_case("IIf") {
            return self.iif(args.unwrap_or(&[]), full);
        }
        if name.eq_ignore_ascii_case("Array") {
            let items = args
                .unwrap_or(&[])
                .iter()
                .map(|a| {
                    let a = self.expr(a);
                    self.coerce(a, &Ty::Object)
                })
                .collect();
            return Expr {
                kind: ExprKind::ArrayLiteral(items),
                ty: Ty::Array(Box::new(Ty::Object), 1),
                range: full,
            };
        }
        if let Some(sig) = builtins::lookup(Owner::Global, &name) {
            return self.builtin(sig, None, args.unwrap_or(&[]), full);
        }
        let message = if let Some(what) = builtins::later_level(&name) {
            format!("`{name}` is not supported at Level 0: it needs {what}")
        } else if name.eq_ignore_ascii_case("Bit") {
            "`Bit` is used through its functions, e.g. `Bit.And(a, b)`".to_string()
        } else if self.records.contains_key(&key(&name)) || builtin_type(&name).is_some() {
            format!("`{name}` is a type, not a value")
        } else {
            format!("`{name}` is not declared")
        };
        self.error(at, message);
        error_expr(full)
    }

    fn index(&mut self, array: Expr, args: &[SyntaxNode], at: TextRange) -> Expr {
        let (elem, rank) = match &array.ty {
            Ty::Array(elem, rank) => ((**elem).clone(), *rank),
            Ty::Error => return error_expr(at),
            other => {
                let other = self.ty_name(other);
                self.error(
                    at,
                    format!("a value of type {other} cannot be indexed or called"),
                );
                return error_expr(at);
            }
        };
        if args.len() != rank as usize {
            self.error(
                at,
                format!(
                    "this array has {rank} dimension(s) but {} index(es) were given",
                    args.len()
                ),
            );
        }
        let indices = args
            .iter()
            .map(|a| {
                let a = self.expr(a);
                self.coerce(a, &Ty::Int)
            })
            .collect();
        Expr {
            kind: ExprKind::Index {
                array: Box::new(array),
                indices,
            },
            ty: elem,
            range: at,
        }
    }

    fn args(&mut self, args: &[SyntaxNode], params: &[Ty], what: &str, at: TextRange) -> Vec<Expr> {
        if args.len() != params.len() {
            self.error(
                at,
                format!(
                    "`{what}` expects {} argument(s) but {} were given",
                    params.len(),
                    args.len()
                ),
            );
        }
        args.iter()
            .zip(params.iter().chain(std::iter::repeat(&Ty::Error)))
            .map(|(a, p)| {
                let a = self.expr(a);
                self.coerce(a, p)
            })
            .collect()
    }

    fn builtin(
        &mut self,
        sig: &Signature,
        receiver: Option<Expr>,
        args: &[SyntaxNode],
        at: TextRange,
    ) -> Expr {
        let params: Vec<Ty> = sig.params.iter().map(|p| p.ty()).collect();
        let args = self.args(args, &params, sig.name, at);
        Expr {
            kind: ExprKind::Builtin {
                builtin: sig.id,
                receiver: receiver.map(Box::new),
                args,
            },
            ty: sig.ret.ty(),
            range: at,
        }
    }

    fn iif(&mut self, args: &[SyntaxNode], at: TextRange) -> Expr {
        if args.len() != 3 {
            self.error(
                at,
                "`IIf` expects 3 arguments: condition, value if true, value if false",
            );
            return error_expr(at);
        }
        let cond = self.condition(Some(&args[0]), at);
        let a = self.expr(&args[1]);
        let a = self.coerce(a, &Ty::Object);
        let b = self.expr(&args[2]);
        let b = self.coerce(b, &Ty::Object);
        Expr {
            kind: ExprKind::IIf(Box::new(cond), Box::new(a), Box::new(b)),
            ty: Ty::Object,
            range: at,
        }
    }

    fn call(&mut self, node: &SyntaxNode) -> Expr {
        let at = range(node);
        let Some(callee) = node.children().next() else {
            return error_expr(at);
        };
        let args = child(node, ARG_LIST).map(|a| exprs(&a)).unwrap_or_default();
        match callee.kind() {
            NAME_REF => self.name_ref(&callee, Some(&args)),
            MEMBER_EXPR => self.member(&callee, Some(&args)),
            _ => {
                let array = self.expr(&callee);
                self.index(array, &args, at)
            }
        }
    }

    /// `receiver.Name`, or `receiver.Name(args)` when `args` is given.
    fn member(&mut self, node: &SyntaxNode, args: Option<&[SyntaxNode]>) -> Expr {
        let at = args.map_or_else(|| range(node), |_| range(&node.parent().unwrap()));
        let children: Vec<SyntaxNode> = node.children().collect();
        let (Some(receiver), Some(member)) = (children.first(), children.last()) else {
            return error_expr(at);
        };
        if children.len() < 2 {
            return error_expr(at);
        }
        let name = name_of(member);
        let receiver_is_bit = receiver.kind() == NAME_REF
            && name_of(receiver).eq_ignore_ascii_case("Bit")
            && self.lookup_var("Bit").is_none();
        if receiver_is_bit {
            return match builtins::lookup(Owner::Bit, &name) {
                Some(sig) => self.builtin(sig, None, args.unwrap_or(&[]), at),
                None => {
                    self.error(range(member), format!("`Bit` has no function `{name}`"));
                    error_expr(at)
                }
            };
        }
        let value = self.expr(receiver);
        if name.eq_ignore_ascii_case("As") {
            return self.inline_cast(value, args, at);
        }
        if let Ty::Record(id) = value.ty {
            let fields = &self.module.record(id).fields;
            if let Some(i) = fields
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(&name))
            {
                let ty = fields[i].ty.clone();
                let field = Expr {
                    kind: ExprKind::Field {
                        record: Box::new(value),
                        field: i as u32,
                    },
                    ty,
                    range: range(node),
                };
                return match args {
                    None => field,
                    Some(args) => self.index(field, args, at),
                };
            }
        }
        let owner = match &value.ty {
            Ty::Error => return error_expr(at),
            Ty::Object => {
                self.error(
                    range(member),
                    format!("cannot use `.{name}` on an Object: assign it to a variable of the right type, or use `.As(<type>)`"),
                );
                return error_expr(at);
            }
            ty => builtins::owner_of(ty),
        };
        let sig = owner.and_then(|o| builtins::lookup(o, &name));
        match sig {
            Some(sig) => self.builtin(sig, Some(value), args.unwrap_or(&[]), at),
            None => {
                let ty = self.ty_name(&value.ty);
                self.error(
                    range(member),
                    format!("{ty} has no member `{name}` at Level 0"),
                );
                error_expr(at)
            }
        }
    }

    /// `x.As(Type)`: an explicit conversion.
    fn inline_cast(&mut self, value: Expr, args: Option<&[SyntaxNode]>, at: TextRange) -> Expr {
        // `.As(Int)`, or `.As(String())` for an array type (parsed as a call).
        let type_name = match args {
            Some([arg]) if arg.kind() == NAME_REF => Some((arg.clone(), false)),
            Some([arg])
                if arg.kind() == CALL_EXPR
                    && arg.children().next().is_some_and(|c| c.kind() == NAME_REF)
                    && child(arg, ARG_LIST).is_some_and(|a| exprs(&a).is_empty()) =>
            {
                Some((arg.children().next().unwrap(), true))
            }
            _ => None,
        };
        let target = match type_name {
            Some((arg, is_array)) => {
                let name = name_of(&arg);
                match builtin_type(&name)
                    .or_else(|| self.records.get(&key(&name)).map(|&id| Ty::Record(id)))
                {
                    Some(ty) if is_array => Ty::Array(Box::new(ty), 1),
                    Some(ty) => ty,
                    None => {
                        self.error(
                            range(&arg),
                            format!(
                                "unknown type `{name}`: library types are not supported at Level 0"
                            ),
                        );
                        return error_expr(at);
                    }
                }
            }
            _ => {
                self.error(at, "`.As` expects a type name: `.As(Int)`");
                return error_expr(at);
            }
        };
        let mut converted = self.coerce(value, &target);
        converted.range = at;
        converted
    }

    /// The original toolchain refuses an arithmetic operator (`+`, `-`, `*`,
    /// `/`, `Mod`) or a negation next to `&` as a syntax error (`"x" & 1 + 2`,
    /// `1 + 2 & "x"`, `"x" & -1`, `"x" & 6 / 2`, `"x" & -6 / 2`; recorded):
    /// an operand of `&` that holds one, outside parentheses or a call's
    /// arguments, needs parentheses.
    fn concat_operand(&mut self, operand: &SyntaxNode) {
        // The first refused operator in the operand's own expression tree.
        let mut nodes = vec![operand.clone()];
        let mut found = None;
        while let Some(node) = nodes.pop() {
            match node.kind() {
                PREFIX_EXPR | BIN_EXPR => {}
                _ => continue,
            }
            let op = tokens(&node).next();
            // A nested `&` is checked on its own.
            if op.as_ref().is_some_and(|t| t.kind() == AMP) {
                continue;
            }
            if let Some(op) =
                op.filter(|t| matches!(t.kind(), PLUS | MINUS | STAR | SLASH | MOD_KW))
            {
                found = Some(op);
                break;
            }
            nodes.extend(exprs(&node));
        }
        let Some(op) = found else {
            return;
        };
        // The fix, when the operand fits on one line.
        let text = operand.text().to_string();
        let text = text.trim();
        let message = if text.contains(['\n', '\r']) {
            format!(
                "`{}` next to `&` needs parentheses around its operand",
                op.text()
            )
        } else {
            format!(
                "`{}` next to `&` needs parentheses: write `({text})`",
                op.text()
            )
        };
        self.error(range(operand), message);
    }

    fn binary(&mut self, node: &SyntaxNode) -> Expr {
        let at = range(node);
        let parts = exprs(node);
        let (Some(l), Some(r)) = (parts.first(), parts.get(1)) else {
            return error_expr(at);
        };
        let Some(op) = tokens(node).next() else {
            return error_expr(at);
        };
        let left = self.expr(l);
        let right = self.expr(r);
        let arith = |op| Some(op);
        let arith_op = match op.kind() {
            PLUS => arith(ArithOp::Add),
            MINUS => arith(ArithOp::Sub),
            STAR => arith(ArithOp::Mul),
            SLASH => arith(ArithOp::Div),
            MOD_KW => arith(ArithOp::Mod),
            _ => None,
        };
        if let Some(op) = arith_op {
            let ty = if op == ArithOp::Div {
                Ty::Double
            } else {
                Ty::arithmetic(&left.ty, &right.ty)
            };
            let left = self.coerce(left, &ty);
            let right = self.coerce(right, &ty);
            return Expr {
                kind: ExprKind::Arith(op, Box::new(left), Box::new(right)),
                ty,
                range: at,
            };
        }
        match op.kind() {
            AMP => {
                for operand in [l, r] {
                    self.concat_operand(operand);
                }
                let left = self.coerce(left, &Ty::String);
                let right = self.coerce(right, &Ty::String);
                Expr {
                    kind: ExprKind::Concat(Box::new(left), Box::new(right)),
                    ty: Ty::String,
                    range: at,
                }
            }
            AND_KW | OR_KW => {
                let op = if op.kind() == AND_KW {
                    LogicOp::And
                } else {
                    LogicOp::Or
                };
                let left = self.coerce(left, &Ty::Boolean);
                let right = self.coerce(right, &Ty::Boolean);
                Expr {
                    kind: ExprKind::Logic(op, Box::new(left), Box::new(right)),
                    ty: Ty::Boolean,
                    range: at,
                }
            }
            EQ | NEQ | LT | GT | LT_EQ | GT_EQ => {
                let cmp = match op.kind() {
                    EQ => CmpOp::Eq,
                    NEQ => CmpOp::Ne,
                    LT => CmpOp::Lt,
                    GT => CmpOp::Gt,
                    LT_EQ => CmpOp::Le,
                    _ => CmpOp::Ge,
                };
                self.compare(cmp, left, right, at)
            }
            _ => error_expr(at),
        }
    }

    /// Chooses how to compare: the left operand's type decides between
    /// numeric and String comparison (as the forum describes for B4X).
    fn compare(&mut self, op: CmpOp, left: Expr, right: Expr, at: TextRange) -> Expr {
        use Ty::*;
        let ordering = !matches!(op, CmpOp::Eq | CmpOp::Ne);
        let (kind, domain) = match (&left.ty, &right.ty) {
            (Error, _) | (_, Error) => return error_expr(at),
            (Null, _) | (_, Null) => {
                let other = if left.ty == Null { &right.ty } else { &left.ty };
                if other.is_primitive() || ordering {
                    let other = self.ty_name(other);
                    self.error(at, format!("cannot compare {other} with Null"));
                    return error_expr(at);
                }
                (CmpKind::Object, Object)
            }
            (l, r) if l.is_numeric() && (r.is_numeric() || matches!(r, String | Object)) => {
                (CmpKind::Numeric, Double)
            }
            (Object, r) if r.is_numeric() => (CmpKind::Numeric, Double),
            (String, r) | (r, String) if r.is_primitive() || matches!(r, String | Object) => {
                (CmpKind::String, String)
            }
            (Char, Char) => (CmpKind::Char, Char),
            (Boolean, Boolean) | (Boolean, Object) | (Object, Boolean) => {
                (CmpKind::Boolean, Boolean)
            }
            (l, r) if l.is_reference() && r.is_reference() && !ordering => {
                (CmpKind::Object, Object)
            }
            (l, r) => {
                let (l, r) = (self.ty_name(l), self.ty_name(r));
                self.error(at, format!("cannot compare {l} with {r}"));
                return error_expr(at);
            }
        };
        if ordering && matches!(kind, CmpKind::Boolean | CmpKind::Object) {
            self.error(
                at,
                "Booleans and objects can only be compared with `=` and `<>`",
            );
            return error_expr(at);
        }
        let left = self.coerce(left, &domain);
        let right = self.coerce(right, &domain);
        Expr {
            kind: ExprKind::Compare(op, kind, Box::new(left), Box::new(right)),
            ty: Boolean,
            range: at,
        }
    }

    fn smart_string(&mut self, node: &SyntaxNode) -> Expr {
        let at = range(node);
        let mut parts = Vec::new();
        for element in node.children_with_tokens() {
            match element {
                NodeOrToken::Token(t) if t.kind() == SMART_STRING_TEXT => {
                    parts.push(SmartPart::Text(t.text().to_string()));
                }
                NodeOrToken::Node(n) if n.kind() == PLACEHOLDER => {
                    let start = tokens(&n)
                        .next()
                        .map(|t| t.text().to_string())
                        .unwrap_or_default();
                    let format = start
                        .trim_start_matches('$')
                        .trim_end_matches('{')
                        .to_ascii_lowercase();
                    let Some(e) = exprs(&n).into_iter().next() else {
                        continue;
                    };
                    let value = self.expr(&e);
                    let target = if format.is_empty() || format == "xml" {
                        Ty::String
                    } else if is_number_format(&format) {
                        Ty::Object
                    } else if matches!(format.as_str(), "date" | "time" | "datetime") {
                        self.error(range(&n), format!("the `${format}{{}}` formatter needs DateTime, which arrives at Level 1"));
                        Ty::Error
                    } else {
                        self.error(
                            range(&n),
                            format!("unknown smart string formatter `${format}{{}}`"),
                        );
                        Ty::Error
                    };
                    let expr = self.coerce(value, &target);
                    parts.push(SmartPart::Value { expr, format });
                }
                _ => {}
            }
        }
        Expr {
            kind: ExprKind::SmartString(parts),
            ty: Ty::String,
            range: at,
        }
    }
}

/// `N` or `N.M` (minimum integers, maximum fractions).
fn is_number_format(format: &str) -> bool {
    let mut parts = format.split('.');
    let ints = parts.next().unwrap_or("");
    let fracs = parts.next();
    parts.next().is_none()
        && !ints.is_empty()
        && ints.bytes().all(|b| b.is_ascii_digit())
        && fracs.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
}
