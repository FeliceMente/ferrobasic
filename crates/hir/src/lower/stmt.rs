//! Subs and statements: statements, loops and assignment targets.

use super::*;

impl Lowerer {
    pub(super) fn block(&mut self, node: &SyntaxNode) -> Block {
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
                // Outside `Class_Globals`, where a loop must use a variable
                // declared with `Dim`, a `For Each` without `As` declares its
                // variable an `Object` (recorded:
                // `for_each_untyped_declared_after`): it loops over a local
                // `Object` that a `Dim` declared before it, or that it
                // declares (recorded: `for_each_untyped_object`,
                // `for_each_untyped_undeclared`), and over a variable of
                // another type it is refused, wherever it is declared
                // (recorded over a local, a parameter and a module
                // variable). Not recorded: over a parameter or a module
                // variable of type `Object`, and the cases
                // `untyped_loop_unrecorded` lists. A Sub's name was judged
                // by `loop_variable`, which lets one through only after the
                // program has an error over it already, and so has a
                // variable of an unknown type.
                if child(node, TYPE_REF).is_none()
                    && !self.in_class_globals
                    && !self.subs.contains_key(&key(&text))
                    && ty != Ty::Error
                {
                    let at = range(&name);
                    let refusal = match var {
                        VarRef::Local(id) if id.0 as usize >= self.params && ty == Ty::Object => {
                            self.untyped_loop_refusal(&text)
                                .map(|(refusal, read)| (refusal.message(), read.unwrap_or(at)))
                        }
                        // Over a parameter `Object`, and over a module variable
                        // `Object` however and wherever it was declared, from a
                        // Sub or in `Process_Globals` (recorded:
                        // `for_each_untyped_param_object`,
                        // `for_each_untyped_module_object`,
                        // `for_each_untyped_pg_loop_object`,
                        // `process_globals_for_each_untyped_object`,
                        // `process_globals_for_each_untyped_after_typed_object`,
                        // `process_globals_for_each_untyped_before_dim_object`,
                        // `process_globals_for_each_untyped_before_typed_object`).
                        _ if ty == Ty::Object => None,
                        _ => self
                            .type_clash(&text, &ty, &Ty::Object, Clash::UntypedForEach)
                            .map(|message| (message, at)),
                    };
                    if let Some((message, at)) = refusal {
                        self.error(at, message);
                        self.refused_loop(node, &text, &name);
                        return;
                    }
                }
                self.accepted_loop(node, &name, &ty);
                let Some(coll) = exprs(node).into_iter().next() else {
                    return;
                };
                let collection = self.expr(&coll);
                let element = match &collection.ty {
                    Ty::Array(elem, 1) => (**elem).clone(),
                    Ty::List | Ty::Iterable => Ty::Object,
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
                    Some(v) if self.ret == Ty::Void && self.in_process_globals => {
                        self.error(
                            v.range,
                            "`Process_Globals` returns nothing, so `Return` cannot return a value",
                        );
                        None
                    }
                    // In `AppStart` the value is evaluated, and nothing can
                    // read it.
                    Some(v) if self.ret == Ty::Void => {
                        push(out, StmtKind::Expr(v));
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
        let refused = self.refused_loop_already(node, name, ty);
        if refused || loop_origin(node) == Origin::UntypedForEach {
            self.sub_named_loops.entry(k).or_default().unrecorded = true;
        }
    }

    /// Whether a loop is refused elsewhere: where its variable is declared
    /// (`refused_loop_vars`); over a variable of an unknown type, reported
    /// where the type is; or, for a `For` over anything but a number or with
    /// `As` in `Process_Globals`, when it is lowered.
    fn refused_loop_already(&self, node: &SyntaxNode, name: &SyntaxNode, ty: &Ty) -> bool {
        self.refused_loop_vars.contains(&range(name))
            || *ty == Ty::Error
            || (node.kind() == FOR_STMT
                && (!ty.is_numeric()
                    || (self.in_process_globals && child(node, TYPE_REF).is_some())))
    }

    /// The variable a `For` / `For Each` loops over, or the loop refused
    /// (reported and checked here). Over a loop variable with a Sub's name,
    /// a loop is accepted or refused as `recorded_loop` says, after the
    /// variable's own rules. After a refused loop, which is an error
    /// already, the loops over the name are not judged.
    fn loop_variable(&mut self, node: &SyntaxNode, text: &str, name: &SyntaxNode) -> LoopVar {
        let found = self.lookup_var(text);
        if let Some((var, ty)) = &found
            && self.is_const(var)
        {
            // B4J: "Cannot modify Const variable." (recorded), for a `For`
            // and a `For Each v As Int` in a Sub over a module `Int` `Const`.
            let refusal = if !self.in_process_globals
                && self.recorded_const(var)
                && *ty == Ty::Int
                && (node.kind() == FOR_STMT || child(node, TYPE_REF).is_some())
            {
                Refusal::Recorded(format!("`{text}` is a `Const`: a loop cannot loop over it"))
            } else if let Some(pair) = self.pair(var) {
                redeclared_const(text, pair, "may be looped over")
            } else {
                Refusal::open(
                    format!("`{text}` is a `Const`"),
                    "a loop may loop over it here",
                    "use another name",
                )
            };
            self.const_error(var, range(name), refusal.message());
            self.refused_loop(node, text, name);
            return LoopVar::Refused;
        }
        let k = key(text);
        if !self.subs.contains_key(&k) {
            return LoopVar::Found(found);
        }
        // A loop variable with a Sub's name: anything else was reported
        // already, or is not one.
        // Whether it is a loop variable of this Sub, or of `Process_Globals`.
        let (own, ty) = match &found {
            Some((VarRef::Global(_), ty)) if self.declared_by_loop(&k) => (false, ty.clone()),
            Some((VarRef::Local(_), ty)) if self.sub_named_local(&k) == Some(true) => {
                let module_ty = self
                    .globals
                    .get(&k)
                    .filter(|_| self.declared_by_loop(&k))
                    .map(|id| self.module.global(*id).ty.clone());
                // Another type than the module variable's: reported already
                // (recorded).
                if module_ty.is_some_and(|module_ty| module_ty != *ty) {
                    return LoopVar::Found(found);
                }
                (true, ty.clone())
            }
            _ => return LoopVar::Found(found),
        };
        let origin = loop_origin(node);
        let before = self.sub_named_loops.get(&k);
        if before.is_some_and(|b| b.unrecorded) || self.refused_loop_already(node, name, &ty) {
            return LoopVar::Found(found);
        }
        if recorded_loop(origin) {
            return LoopVar::Found(found);
        }
        let what = if own {
            "a loop variable of this Sub"
        } else {
            "a module variable"
        };
        self.error(
            range(name),
            format!(
                "`{text}` is {what} with the name of a Sub: a `For Each` without `As` cannot \
                 loop over it"
            ),
        );
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

    pub(super) fn condition(&mut self, node: Option<&SyntaxNode>, at: TextRange) -> Expr {
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
                let refused_dim = self.sub_named_local(&key(&name)) == Some(false);
                let found = if refused_dim {
                    self.lookup_var(&name)
                } else {
                    self.lookup_value(&name)
                };
                // A `Const` then a `Dim` of its name in `Process_Globals`
                // make a variable a Sub may assign (recorded).
                let assignable = |this: &Self, var: &VarRef| {
                    !this.in_process_globals && this.pair(var) == Some(Pair::ConstDim)
                };
                match found {
                    Some((var, ty))
                        if !refused_dim && self.is_const(&var) && !assignable(self, &var) =>
                    {
                        // B4J: "Cannot modify Const variable." (recorded), also
                        // when the `Const` follows a `Dim` of the name.
                        let name = self.var(&var).name.clone();
                        let pair = self.pair(&var);
                        let recorded_pair =
                            !self.in_process_globals && pair == Some(Pair::DimConst);
                        let message = if let Some(pair) = pair
                            && !recorded_pair
                        {
                            redeclared_const(&name, pair, "may be assigned").message()
                        } else {
                            format!("`{name}` is a `Const`: it cannot be assigned")
                        };
                        self.const_error(&var, range(node), message);
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
                if let Some((var, message)) = self.const_place(&expr) {
                    self.const_error(&var, range(node), message);
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
