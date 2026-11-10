//! Constants.

use super::*;

impl Lowerer {
    pub(super) fn var(&self, var: &VarRef) -> &Var {
        match var {
            VarRef::Global(id) => self.module.global(*id),
            VarRef::Local(id) => &self.locals[id.0 as usize],
        }
    }

    pub(super) fn is_const(&self, var: &VarRef) -> bool {
        self.var(var).is_const
    }

    /// Makes the variable a declarator of a `Const` declares a constant,
    /// when this declarator is its first declaration.
    pub(super) fn mark_const(&mut self, decl: &SyntaxNode, name: &str) {
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

    /// Whether B4J's answers recorded for a constant apply to it: one of a
    /// main module's `Process_Globals`, declared there once, by its `Const`
    /// alone.
    pub(super) fn recorded_const(&self, var: &VarRef) -> bool {
        self.class_globals.is_none() && self.const_state(var) == Some(ConstState::Single)
    }

    fn const_state(&self, var: &VarRef) -> Option<ConstState> {
        match var {
            VarRef::Global(id) => self.const_states.get(id).copied(),
            VarRef::Local(_) => None,
        }
    }

    /// Reports an error about changing a constant, unless declaring it
    /// again was refused already. Whether one declared both ways may change
    /// is asked once: after that, the program has an error already.
    pub(super) fn const_error(&mut self, var: &VarRef, at: TextRange, message: String) {
        match (var, self.const_state(var)) {
            (_, Some(ConstState::Reported | ConstState::Redeclared { reported: true, .. })) => {}
            (
                VarRef::Global(id),
                Some(ConstState::Redeclared {
                    reported: false,
                    pair,
                }),
            ) => {
                self.const_states.insert(
                    *id,
                    ConstState::Redeclared {
                        reported: true,
                        pair,
                    },
                );
                self.error(at, message);
            }
            _ => self.error(at, message),
        }
    }

    /// The two declarations of a name declared twice in `Process_Globals`,
    /// if it was: whether it may change is then not known.
    pub(super) fn pair(&self, var: &VarRef) -> Option<Pair> {
        match self.const_state(var) {
            Some(ConstState::Redeclared { pair, .. }) => Some(pair),
            _ => None,
        }
    }

    /// Checks a declarator of a `Dim` against the constants, and whether
    /// to lower it. A `Const` must be given its value there (B4J: "Const
    /// variable must be set in the declaration line.", recorded, with
    /// bounds too). In `Process_Globals`, an `Int` declared by a `Const`
    /// and by a `Dim` with a value, in either order, is built, each
    /// declaration giving its value in turn (recorded); a `Dim` in a Sub
    /// with the name of a module `Int` `Const`, with a value, is refused
    /// (B4J: "Cannot redeclare a const variable.", recorded). Any other
    /// declaration again of a constant's name, of the same type, is not
    /// recorded; with another type, the clash is reported already.
    pub(super) fn const_declaration(&mut self, decl: &SyntaxNode, ty: &Ty, var: &VarRef) -> bool {
        let is_const = in_const(decl);
        let has_value = !exprs(decl).is_empty();
        let declared = self.var(var);
        let name = child(decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
        let at = child(decl, NAME).map_or_else(|| range(decl), |n| range(&n));
        let same_type = declared.ty == *ty;
        if declared.range != range(decl) && same_type && (is_const || declared.is_const) {
            let global = matches!(var, VarRef::Global(_));
            // `declare_process_globals` decided, and reported, those of
            // `Process_Globals`; any it did not is refused below.
            if self.in_process_globals
                && !self.in_class_globals
                && global
                && let Some(&lowered) = self.const_declarations.get(&range(decl))
            {
                return lowered;
            }
            let state = self.const_state(var);
            if state == Some(ConstState::Reported) {
                return false;
            }
            let message = if !self.in_process_globals
                && self.recorded_const(var)
                && *ty == Ty::Int
                && !is_const
                && has_value
            {
                format!("`{name}` is a `Const` of the module: a `Dim` cannot declare it again")
            } else if let Some(pair) = self.pair(var) {
                redeclared_const(&name, pair, "may be declared again").message()
            } else {
                const_again_open(&name, is_const)
            };
            // Over a variable declared both ways, this asks its question.
            self.const_error(var, at, message);
            return false;
        }
        if is_const && !has_value {
            self.error(
                at,
                format!("`{name}` is a `Const`: give it its value where it is declared"),
            );
            return false;
        }
        true
    }

    /// Why an item or field assignment is refused when it changes a
    /// constant: an item of a `Const` array, or a field of a `Const` record,
    /// is refused (B4J: "Cannot modify Const variable.", recorded); an item
    /// of an item, a field of a field, or any part of a constant declared
    /// again, is not recorded.
    pub(super) fn const_place(&self, expr: &Expr) -> Option<(VarRef, String)> {
        let item = matches!(&expr.kind, ExprKind::Index { array, .. }
            if matches!(array.kind, ExprKind::Var(_)));
        let field = matches!(&expr.kind, ExprKind::Field { record, .. }
            if matches!(&record.kind, ExprKind::Var(var) if self.recorded_const(var)));
        let mut root = expr;
        loop {
            match &root.kind {
                ExprKind::Index { array, .. } => root = array,
                ExprKind::Field { record, .. } => root = record,
                ExprKind::Var(var) if self.is_const(var) => {
                    let name = &self.var(var).name;
                    return Some((
                        *var,
                        if let Some(pair) = self.pair(var) {
                            redeclared_const(name, pair, "may change").message()
                        } else if item {
                            format!("`{name}` is a `Const`: its items cannot be assigned")
                        } else if field {
                            format!("`{name}` is a `Const`: its fields cannot be assigned")
                        } else {
                            Refusal::asked(
                                format!("`{name}` is a `Const`"),
                                "this part of it may be assigned",
                            )
                            .message()
                        },
                    ));
                }
                _ => return None,
            }
        }
    }
}
