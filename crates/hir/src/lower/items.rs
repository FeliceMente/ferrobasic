//! Module level.

use super::*;

impl Lowerer {
    pub(super) fn error(&mut self, range: TextRange, message: impl Into<String>) {
        self.diags.push(Diagnostic {
            range,
            message: message.into(),
        });
    }

    /// The one check that a declaration again of `name` keeps its type:
    /// `None` when the types agree, or one of them is an error reported
    /// already; otherwise the message.
    pub(super) fn type_clash(
        &self,
        name: &str,
        existing: &Ty,
        new: &Ty,
        clash: Clash,
    ) -> Option<String> {
        if existing == new || *existing == Ty::Error || *new == Ty::Error {
            return None;
        }
        let was = || self.ty_name(existing);
        let now = || self.ty_name(new);
        Some(match clash {
            Clash::InSub => format!("`{name}` is already declared as {} in this Sub", was()),
            Clash::Global => format!("`{name}` is already declared with another type"),
            Clash::DimAfterLoop => format!(
                "`{name}` is already declared as {} by a loop of `Process_Globals`: this `Dim` \
                 cannot declare it again as {}",
                was(),
                now()
            ),
            Clash::LoopInProcessGlobals => format!(
                "`{name}` is already declared as {} in `Process_Globals`: this loop cannot \
                 declare it again as {}",
                was(),
                now()
            ),
            Clash::LocalOfModuleVariable => format!(
                "`{name}` is a module variable of type {}: a local cannot declare it again as {}",
                was(),
                now()
            ),
            Clash::UntypedForEach => format!(
                "`{name}` is already declared as {}: a `For Each` without `As` declares its \
                 variable an Object, so it cannot loop over it; write `For Each {name} As \
                 <type> In ...`",
                was()
            ),
        })
    }

    pub(super) fn ty_name(&self, ty: &Ty) -> String {
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

    pub(super) fn module_items(&mut self, root: &SyntaxNode) {
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
    pub(super) fn inline_java(&mut self, node: &SyntaxNode) {
        let start = range(node).start();
        let text = node.text().to_string();
        let first = text.trim_start().split(['\r', '\n']).next().unwrap_or("");
        let first = first.split('\'').next().unwrap_or("").trim_end();
        self.error(
            TextRange::at(start, (first.len() as u32).into()),
            "inline Java (`#If Java`) is not supported at Level 0: it needs Java interop (Level 3)",
        );
    }

    pub(super) fn unsupported_directive(&mut self, node: &SyntaxNode) {
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

    pub(super) fn resolve_type(&mut self, type_ref: &SyntaxNode) -> Ty {
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
        // A Sub without `As` returns a `String`, as B4J declares it: the
        // value of `Return 5` prints `5` (recorded:
        // `untyped_sub_return_value_used`), and one that returns nothing
        // returns `""`, the `return "";` B4J generates. `Process_Globals`
        // and `AppStart`, which only B4J's runtime calls, return nothing.
        let ret = match child(node, TYPE_REF) {
            Some(t) => self.resolve_type(&t),
            None if matches!(key(&name).as_str(), "process_globals" | "appstart") => Ty::Void,
            None => Ty::String,
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
                        Refusal::open(
                            format!("`{name}` is a Sub"),
                            "a variable of `Class_Globals` may have its name",
                            "use another name",
                        )
                        .message(),
                    );
                    self.declare_global(name, Ty::Error, range(&decl), Origin::Dim);
                    continue;
                }
                let origin = Origin::of_declarator(&decl);
                self.declare_global(name.clone(), ty, range(&decl), origin);
                self.mark_const(&decl, &name);
            }
        }
    }

    pub(super) fn declare_global(&mut self, name: String, ty: Ty, at: TextRange, origin: Origin) {
        match self.globals.get(&key(&name)) {
            Some(&id) => {
                let existing = &self.module.global(id).ty;
                if let Some(message) = self.type_clash(&name, existing, &ty, Clash::Global) {
                    self.error(at, message);
                }
            }
            None => {
                let id = GlobalId(self.module.globals.len() as u32);
                self.globals.insert(key(&name), id);
                self.global_origins.insert(key(&name), origin);
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
        // Each name's declarations: `Err` for a loop, at its name.
        let mut declarations: HashMap<String, Vec<Result<GlobalDeclaration, TextRange>>> =
            HashMap::new();
        for found in in_source_order(sub, false) {
            if let Found::Dim(node) = found {
                let direct = node.parent().and_then(|block| block.parent()).as_ref() == Some(sub);
                let in_if = node
                    .parent()
                    .and_then(|block| block.parent())
                    .filter(|n| n.kind() == IF_STMT)
                    .and_then(|stmt| stmt.parent())
                    .and_then(|block| block.parent())
                    .as_ref()
                    == Some(sub);
                for (decl, ty) in self.declarators(&node) {
                    let name = child(&decl, NAME).map(|n| name_of(&n)).unwrap_or_default();
                    let declaration = GlobalDeclaration {
                        range: range(&decl),
                        at: child(&decl, NAME).map_or_else(|| range(&decl), |n| range(&n)),
                        ty: ty.clone(),
                        is_const: in_const(&decl),
                        literal: exprs(&decl).first().is_some_and(|e| e.kind() == LITERAL),
                        direct,
                        in_if,
                    };
                    let origin = Origin::of_declarator(&decl);
                    self.declare_dim_global(name.clone(), ty, range(&decl), origin);
                    self.mark_const(&decl, &name);
                    declarations
                        .entry(key(&name))
                        .or_default()
                        .push(Ok(declaration));
                }
            } else if let Found::Loop {
                name,
                origin,
                type_ref,
                ..
            } = found
            {
                self.declare_loop_global(&name, origin, type_ref.as_ref());
                declarations
                    .entry(key(&name_of(&name)))
                    .or_default()
                    .push(Err(range(&name)));
            }
        }
        for (k, found) in declarations {
            let Some(&id) = self.globals.get(&k) else {
                continue;
            };
            let global = self.module.global(id);
            let (first, is_const, ty) = (global.range, global.is_const, global.ty.clone());
            let name = global.name.clone();
            // The recorded pairs: a `Const` of an `Int` or a `String` with a
            // literal value, then a `Dim` of its type, with a value or not,
            // a statement of `Process_Globals` or of an `If` there
            // (`const_dim_after*`); an `Int` declared by a `Dim`, then by a
            // `Const` (`const_after_dim`), or by two `Const`s (`const_twice`),
            // both with a literal value and statements of `Process_Globals`.
            let pair = match &found[..] {
                [Ok(a), Ok(b), ..] if a.ty == b.ty => match (a.is_const, b.is_const) {
                    (true, false)
                        if a.literal
                            && a.direct
                            && (b.direct || b.in_if)
                            && matches!(a.ty, Ty::Int | Ty::String) =>
                    {
                        Some(Pair::ConstDim)
                    }
                    (false, true) | (true, true)
                        if a.literal && b.literal && a.direct && b.direct && a.ty == Ty::Int =>
                    {
                        Some(if a.is_const {
                            Pair::ConstConst
                        } else {
                            Pair::DimConst
                        })
                    }
                    _ => None,
                },
                _ => None,
            };
            // Every declaration but the first: lowered for the recorded pair
            // alone (the program has an error otherwise).
            let again: Vec<TextRange> = found
                .iter()
                .flatten()
                .map(|d| d.range)
                .filter(|&r| r != first)
                .collect();
            if let Some(pair) = pair
                && found.len() == 2
            {
                self.module.globals[id.0 as usize].is_const = true;
                self.const_states.insert(
                    id,
                    ConstState::Redeclared {
                        reported: false,
                        pair,
                    },
                );
                self.const_declarations
                    .extend(again.iter().map(|&r| (r, true)));
                continue;
            }
            if pair.is_some() {
                // The pair, then what makes it open: reported there; another
                // type is the clash reported already.
                match &found[2] {
                    Ok(d) if d.ty == ty => {
                        self.error(
                            d.at,
                            redeclared_const(&name, "may be declared again").message(),
                        );
                    }
                    Ok(_) => {}
                    Err(at) => {
                        self.error(*at, redeclared_const(&name, "may be looped over").message())
                    }
                }
                self.const_states.insert(id, ConstState::Reported);
                self.const_declarations
                    .extend(again.iter().map(|&r| (r, false)));
                continue;
            }
            // The first declaration again that is refused, reported here so
            // that every Sub sees it refused, wherever it is; another type
            // is the clash reported already.
            let refused = found
                .iter()
                .flatten()
                .find(|d| d.range != first && d.ty == ty && (d.is_const || is_const));
            if let Some(d) = refused {
                self.error(d.at, const_again_open(&name, d.is_const));
                self.const_states.insert(id, ConstState::Reported);
                self.const_declarations
                    .extend(again.iter().map(|&r| (r, false)));
            } else if let [Ok(d)] = &found[..]
                && d.is_const
                && d.direct
            {
                self.const_states.insert(id, ConstState::Single);
            }
        }
        self.declare_untyped_loop_globals(sub);
    }

    /// A `For Each` without `As` in `Process_Globals` over a name nothing
    /// else there declares makes it a module variable `Object`, which every
    /// Sub sees (recorded: `process_globals_for_each_untyped_undeclared`).
    /// Not recorded, so left undeclared and refused when lowered: over the
    /// name of a Sub, a built-in or `Bit`, after a use or an
    /// assignment of the name there, and with a `For … As` over it there.
    fn declare_untyped_loop_globals(&mut self, sub: &SyntaxNode) {
        let mut seen = HashSet::new();
        let mut skip = HashSet::new();
        let mut loops = Vec::new();
        for found in in_source_order(sub, true) {
            match found {
                Found::Use(node) | Found::Assign(node) => {
                    seen.insert(key(&name_of(&node)));
                }
                Found::Loop {
                    name,
                    origin: Origin::UntypedForEach,
                    ..
                } => {
                    if seen.contains(&key(&name_of(&name))) {
                        skip.insert(key(&name_of(&name)));
                    }
                    loops.push(name);
                }
                Found::Loop {
                    name,
                    origin: Origin::TypedCounter,
                    ..
                } => {
                    skip.insert(key(&name_of(&name)));
                }
                _ => {}
            }
        }
        for name in loops {
            let text = name_of(&name);
            let k = key(&text);
            if !self.globals.contains_key(&k)
                && !skip.contains(&k)
                && !self.refused_loop_names.contains(&k)
                // A Type's name can also declare a variable (run 21:
                // process_globals_for_each_untyped_named_like_type).
                && !self.subs.contains_key(&k)
                && builtins::lookup(Owner::Global, &text).is_none()
                && !text.eq_ignore_ascii_case("Bit")
            {
                self.declare_global(text, Ty::Object, range(&name), Origin::UntypedForEach);
            }
        }
    }

    /// Whether `text` names nothing but a variable: not a Sub, a `Type` or a
    /// built-in. A loop or an assignment declares only such a name.
    pub(super) fn plain_name(&self, text: &str) -> bool {
        let k = key(text);
        !self.subs.contains_key(&k)
            && !self.records.contains_key(&k)
            && builtins::lookup(Owner::Global, text).is_none()
    }

    /// A `Dim` of `Process_Globals`. After a loop that declared the name, the
    /// same type is that variable, as after a `Dim`; another type is refused.
    fn declare_dim_global(&mut self, name: String, ty: Ty, at: TextRange, origin: Origin) {
        let k = key(&name);
        if self.subs.contains_key(&k) {
            // B4J: "Variable name cannot be the same as a sub name" (recorded).
            self.error(at, variable_named_like_sub(&name));
            return;
        }
        if let Some(&id) = self.globals.get(&k)
            && self.declared_by_loop(&k)
        {
            let existing = &self.module.global(id).ty;
            if let Some(message) = self.type_clash(&name, existing, &ty, Clash::DimAfterLoop) {
                self.error(at, message);
            }
            return;
        }
        self.declare_global(name, ty, at, origin);
    }

    /// Whether a loop of `Process_Globals` declared this module variable.
    pub(super) fn declared_by_loop(&self, k: &str) -> bool {
        self.global_origins.get(k).is_some_and(|o| o.is_loop())
    }

    fn declare_loop_global(
        &mut self,
        name: &SyntaxNode,
        origin: Origin,
        type_ref: Option<&SyntaxNode>,
    ) {
        let text = name_of(name);
        let k = key(&text);
        let ty = match (origin, type_ref) {
            (Origin::Counter, _) => Ty::Int,
            (Origin::TypedForEach, Some(t)) => self.resolve_type(t),
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
            let counter_over_dim = origin == Origin::Counter && !self.declared_by_loop(&k);
            let existing = &self.module.global(id).ty;
            if !counter_over_dim
                && let Some(message) =
                    self.type_clash(&text, existing, &ty, Clash::LoopInProcessGlobals)
            {
                self.refused_loop_names.insert(k);
                self.error(range(name), message);
            }
            return;
        }
        if self.loop_name_free(&text, name) {
            self.declare_global(text, ty, range(name), origin);
        }
    }

    /// Whether a loop variable that `Process_Globals` declares may take this
    /// name. A Sub's may (recorded: `lookup_value`). `Bit` may not: B4J's
    /// compiler fails on it (recorded:
    /// `process_globals_loop_named_like_bit`). A `Type`'s may: B4J builds
    /// it (recorded: `process_globals_loop_named_like_type`). Whether it
    /// hides a keyword of the same name is not recorded, so that is
    /// refused, once.
    fn loop_name_free(&mut self, text: &str, name: &SyntaxNode) -> bool {
        if text.eq_ignore_ascii_case("Bit") {
            self.refused_loop_names.insert(key(text));
            self.error(
                range(name),
                format!(
                    "the loop variable `{text}` of `Process_Globals` cannot have the name of \
                     the `Bit` object: use another name"
                ),
            );
            return false;
        }
        if builtins::lookup(Owner::Global, text).is_none() {
            return true;
        }
        self.refused_loop_names.insert(key(text));
        self.error(
            range(name),
            Refusal::open(
                format!(
                    "the loop variable `{text}` of `Process_Globals` has the name of a keyword"
                ),
                "it would hide it",
                "use another name",
            )
            .message(),
        );
        false
    }

    /// A loop variable `declare_loop_globals` already reported.
    pub(super) fn refused(&self, name: &str) -> bool {
        self.in_process_globals
            && !self.in_class_globals
            && self.refused_loop_names.contains(&key(name))
    }

    pub(super) fn globals_sub(&self) -> &'static str {
        if self.in_class_globals {
            "Class_Globals"
        } else {
            "Process_Globals"
        }
    }

    /// The declarators of a `Dim` with their types: a declarator without
    /// `As` takes the type of the next `As` to its right; with none, String.
    pub(super) fn declarators(&mut self, dim: &SyntaxNode) -> Vec<(SyntaxNode, Ty)> {
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
