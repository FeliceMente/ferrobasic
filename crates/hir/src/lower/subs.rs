//! Subs and statements: Subs, their locals and hoisting.

use super::*;

impl Lowerer {
    pub(super) fn lower_sub(&mut self, id: SubId, node: &SyntaxNode) {
        let sub = &self.module.subs[id.0 as usize];
        self.locals = sub.locals.clone();
        self.local_names = self
            .locals
            .iter()
            .enumerate()
            .map(|(i, v)| (key(&v.name), LocalId(i as u32)))
            .collect();
        self.ret = sub.ret.clone();
        self.local_history = self
            .locals
            .iter()
            .map(|v| (key(&v.name), LocalHistory::param()))
            .collect();
        self.sub_named_loops.clear();
        self.in_class_globals = self.class_globals == Some(id);
        self.in_process_globals = self.module.process_globals == Some(id) || self.in_class_globals;
        self.loops = 0;
        self.exits = 0;
        self.params = self.module.sub(id).params as usize;
        if !self.in_process_globals {
            // B4J: "Parameter name cannot hide global variable name" (recorded).
            let count = self.params;
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

    pub(super) fn declare_local(&mut self, name: &str, ty: Ty, at: TextRange) {
        match self.local_names.get(&key(name)) {
            Some(&id) => {
                let existing = &self.locals[id.0 as usize].ty;
                if let Some(message) = self.type_clash(name, existing, &ty, Clash::InSub) {
                    self.refused_loop_vars.insert(at);
                    self.error(at, message);
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
            return;
        }
        if !self.local_names.contains_key(&k)
            && !self.subs.contains_key(&k)
            && let Some(&id) = self.globals.get(&k)
            && self.module.global(id).ty == ty
        {
            return;
        }
        self.declare_local(name, ty, at);
    }

    /// The counter a `For` of a Sub declares. With a Sub's name an undeclared
    /// counter is a local whose uses mean the Sub, and a typed one is refused
    /// (B4J: "Regular variable followed by '=' expected", recorded).
    fn declare_loop_local(&mut self, name: &str, ty: Ty, at: TextRange, typed: bool) {
        let k = key(name);
        if typed && self.subs.contains_key(&k) {
            // Whatever declared the name first (recorded without one).
            self.refused_loop_vars.insert(at);
            self.error(
                at,
                format!(
                    "`{name}` is a Sub: a `For` counter with its name cannot be declared \
                     with `As`, so loop without `As`"
                ),
            );
            if !self.local_names.contains_key(&k) {
                self.push_local(name, ty, at);
            }
            return;
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
        if existing == *ty && self.subs.contains_key(&key(name)) {
            return;
        }
        let message = if existing == *ty {
            Refusal::open(
                format!(
                    "`{name}` is also a module variable ({})",
                    self.ty_name(&existing)
                ),
                "this loop declares a new local or means the module variable",
                "loop over it without `As`",
            )
            .message()
        } else {
            let Some(clash) = self.type_clash(name, &existing, ty, Clash::LocalOfModuleVariable)
            else {
                return;
            };
            clash
        };
        self.refused_loop_vars.insert(at);
        // After a constant refused, or asked about once, no error of its own.
        self.const_error(&VarRef::Global(id), at, message);
    }

    /// Declares a local name with `declare`, in source order, and notes
    /// the declaration: the first one that makes it a local of this Sub (a
    /// `Dim` of a module variable's name and type makes none), then any
    /// `Dim`.
    fn declare_noted(
        &mut self,
        name: &str,
        origin: Origin,
        before: &SoFar,
        declare: impl FnOnce(&mut Self),
    ) {
        let k = key(name);
        let fresh = !self.local_names.contains_key(&k);
        declare(self);
        match self.local_history.get_mut(&k) {
            Some(history) => {
                history.dimmed |= matches!(origin, Origin::Dim | Origin::Const);
            }
            None if fresh && self.local_names.contains_key(&k) => {
                let history = LocalHistory::new(origin, before, &k);
                self.local_history.insert(k, history);
            }
            None => {}
        }
    }

    /// Why a `For Each` without `As` over a local `Object` of this Sub that
    /// it declared is refused, if it is. After an assignment of the name,
    /// which declares a `String`, or a read of it, B4J refuses it
    /// (recorded: `for_each_untyped_assigned_before`,
    /// `for_each_untyped_read_before`). Declared again as an `Object` after
    /// it, by a `Dim` with or without a value or by a `For Each … As
    /// Object`, it builds, as one variable (recorded:
    /// `for_each_untyped_object_declared_after`,
    /// `for_each_untyped_then_object_valued`,
    /// `for_each_untyped_then_typed_object`). Returns the
    /// refusal, and where it is reported when that is not at the loop: B4J
    /// reports a read at the read.
    pub(super) fn untyped_loop_refusal(&self, text: &str) -> Option<(Refusal, Option<TextRange>)> {
        let history = self.local_history.get(&key(text))?;
        if history.first != Origin::UntypedForEach {
            return None;
        }
        if history.assigned_before {
            let clash = self.type_clash(text, &Ty::String, &Ty::Object, Clash::UntypedForEach)?;
            Some((Refusal::Recorded(clash), None))
        } else if let Some(read) = history.used_before {
            // B4J: "Undeclared variable 'u' is used before it was assigned
            // any value." (recorded: `for_each_untyped_read_before`).
            let refusal = Refusal::Recorded(format!(
                "`{text}` is read here before a later `For Each` declares it, and before it is \
                 assigned a value: declare it first, e.g. `Dim {text} As Object`"
            ));
            Some((refusal, Some(read)))
        } else {
            None
        }
    }

    /// Every local of a Sub is visible in the whole Sub: declare them all first.
    fn hoist_locals(&mut self, body: &SyntaxNode) {
        let found: Vec<Found> = in_source_order(body, true).collect();
        // The names a `Dim` or a loop declares anywhere in the Sub: an
        // assignment above one of them assigns that variable, as B4X hoists
        // declarations.
        let declared: HashSet<String> = found
            .iter()
            .flat_map(|f| match f {
                Found::Dim(node) => node
                    .descendants()
                    .filter(|d| d.kind() == DECLARATOR)
                    .filter_map(|d| child(&d, NAME))
                    .map(|n| key(&name_of(&n)))
                    .collect::<Vec<_>>(),
                Found::Loop { name, .. } => vec![key(&name_of(name))],
                Found::Assign(_) | Found::Use(_) => Vec::new(),
            })
            .collect();
        // In source order: the names used and assigned so far.
        let mut so_far = SoFar::default();
        for f in found {
            match f {
                Found::Use(node) => {
                    so_far
                        .used
                        .entry(key(&name_of(&node)))
                        .or_insert(range(&node));
                }
                Found::Dim(node) => {
                    for (decl, ty) in self.declarators(&node) {
                        let name = child(&decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
                        let origin = Origin::of_declarator(&decl);
                        self.declare_noted(&name, origin, &so_far, |this| {
                            this.declare_dim_local(&name, ty, range(&decl), true)
                        });
                        self.mark_const(&decl, &name);
                    }
                }
                Found::Loop {
                    name,
                    origin,
                    type_ref,
                    ..
                } => {
                    let text = name_of(&name);
                    match origin {
                        Origin::TypedForEach | Origin::TypedCounter => {
                            let ty = type_ref.map_or(Ty::Error, |t| self.resolve_type(&t));
                            self.declare_noted(&text, origin, &so_far, |this| {
                                if origin == Origin::TypedForEach {
                                    this.declare_dim_local(&text, ty, range(&name), false);
                                } else {
                                    this.declare_loop_local(&text, ty, range(&name), true);
                                }
                            });
                        }
                        // Not declared before it, in source order, the
                        // variable of a `For Each` without `As` is an
                        // `Object`: a later `Dim` of another type is refused
                        // ("Previous: Object", recorded:
                        // `for_each_untyped_declared_after`), and a name
                        // nothing declares builds (recorded:
                        // `for_each_untyped_undeclared`). After an
                        // assignment of the name, it is refused
                        // (`untyped_loop_refusal`).
                        Origin::UntypedForEach => {
                            if self.lookup_var(&text).is_none() && self.plain_name(&text) {
                                self.declare_noted(&text, origin, &so_far, |this| {
                                    this.declare_local(&text, Ty::Object, range(&name))
                                });
                            }
                        }
                        // A counter not declared before it, in source order,
                        // is an Int, which a later declaration of another
                        // type cannot change (B4J: "Current declaration does
                        // not match previous one", recorded:
                        // `local_typed_for_each_other_type_after_counter_named_like_sub`).
                        _ => {
                            if self.lookup_var(&text).is_none() {
                                self.declare_noted(&text, origin, &so_far, |this| {
                                    this.declare_loop_local(&text, Ty::Int, range(&name), false)
                                });
                            }
                        }
                    }
                }
                // An assignment to a name nothing in the Sub, nor the
                // module, declares, and that is not a Sub, a built-in or a
                // `Type`, declares it: B4J builds `Undeclared = 5`
                // (recorded: `undeclared_variable`). Its type is a `String`
                // (recorded: `undeclared_variable_type`).
                Found::Assign(target) => {
                    let text = name_of(&target);
                    if self.lookup_var(&text).is_none()
                        && !declared.contains(&key(&text))
                        && self.plain_name(&text)
                    {
                        self.declare_noted(&text, Origin::Assignment, &so_far, |this| {
                            this.declare_local(&text, Ty::String, range(&target))
                        });
                    }
                    so_far.assigned.insert(key(&text));
                }
            }
        }
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<(VarRef, Ty)> {
        let k = key(name);
        if let Some(&id) = self.local_names.get(&k) {
            return Some((VarRef::Local(id), self.locals[id.0 as usize].ty.clone()));
        }
        self.globals
            .get(&k)
            .map(|&id| (VarRef::Global(id), self.module.global(id).ty.clone()))
    }
}
