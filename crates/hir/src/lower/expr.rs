//! Expressions.

use super::*;

impl Lowerer {
    /// Converts `expr` to `to`, inserting an explicit `Convert`.
    pub(super) fn coerce(&mut self, expr: Expr, to: &Ty) -> Expr {
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

    pub(super) fn expr(&mut self, node: &SyntaxNode) -> Expr {
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
    pub(super) fn lookup_value(&self, name: &str) -> Option<(VarRef, Ty)> {
        let found = self.lookup_var(name)?;
        let k = key(name);
        let sub_named = self.subs.contains_key(&k)
            && match found.0 {
                VarRef::Global(_) => self.declared_by_loop(&k),
                VarRef::Local(_) => self.sub_named_local(&k).is_some(),
            };
        (!sub_named).then_some(found)
    }

    /// A local of the current Sub with a Sub's name that a `Dim` or a loop
    /// declares: `true` when a loop declared it first, `false` for a
    /// refused `Dim`. Its uses mean the Sub (recorded).
    pub(super) fn sub_named_local(&self, k: &str) -> Option<bool> {
        if !self.subs.contains_key(k) {
            return None;
        }
        let history = self.local_history.get(k)?;
        if history.first.is_loop() {
            Some(true)
        } else if history.dimmed {
            Some(false)
        } else {
            None
        }
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
                // B4J's parser takes one comparison, then expects what
                // follows it: "'Then' expected" after `If a = b = c`,
                // "')' expected" in `Log(3 < 2 < 1)` (recorded:
                // `chained_comparison_equal`, `chained_comparison_less`).
                if [l, r].iter().any(|o| is_comparison(o)) {
                    self.error(
                        op.text_range(),
                        "comparisons cannot be chained: put one of them in parentheses",
                    );
                    return error_expr(at);
                }
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

    /// Chooses how to compare. `=` and `<>`: the left operand's type
    /// decides between numeric and String comparison (recorded:
    /// `mixed_compare_number_left`, `mixed_compare_string_left`). `<`, `>`,
    /// `<=` and `>=` compare numbers: B4J parses a String operand with
    /// `Double.parseDouble`, so `"a" < "b"` raises `NumberFormatException`
    /// (recorded: `string_compare_less`, `mixed_compare_string_left`), and
    /// refuses a Boolean or a Char ("Cannot cast type ... to: number",
    /// recorded: `boolean_less`, `char_string_compare`).
    fn compare(&mut self, op: CmpOp, left: Expr, right: Expr, at: TextRange) -> Expr {
        use Ty::*;
        let ordering = !matches!(op, CmpOp::Eq | CmpOp::Ne);
        let (kind, domain) = match (&left.ty, &right.ty) {
            (Error, _) | (_, Error) => return error_expr(at),
            // `n = Null`: B4J converts the `Null` with `BA.ObjectToNumber`,
            // which parses its text, `"null"`: a `NumberFormatException`
            // when it runs, whatever the number's type (recorded:
            // `null_number_compare`, `double_eq_null`, `long_eq_null`,
            // `number_types_eq_null`).
            (l, Null) if l.is_numeric() && !ordering => {
                let right = self.coerce(right, &String);
                let left = self.coerce(left, &Double);
                let right = self.coerce(right, &Double);
                return Expr {
                    kind: ExprKind::Compare(op, CmpKind::Numeric, Box::new(left), Box::new(right)),
                    ty: Boolean,
                    range: at,
                };
            }
            (Char, _) | (_, Char) if ordering => {
                self.error(
                    at,
                    "a Char cannot be compared with `<`, `>`, `<=` or `>=`: those compare \
                     numbers, and a Char does not convert to one",
                );
                return error_expr(at);
            }
            // `c = 65`: B4J converts the number with `BA.ObjectToChar`, the
            // first character of its text, `'6'` (generated
            // `_c==BA.ObjectToChar(65)` and recording: `char_number_compare`;
            // a `Long`, `Short`, `Byte`, `Double` and `Float` too:
            // `char_eq_long`, `char_eq_double`, `char_eq_float`).
            // `ObjectToChar` takes an `Object`: every number is boxed, and
            // its own `toString` gives the text (`1.0E-5` for a Float, not
            // B4J's text of it widened to a Double).
            (Char, Int | Long | Short | Byte | Double | Float) if !ordering => {
                let right = self.coerce(right, &Object);
                let right = self.coerce(right, &String);
                let right = self.coerce(right, &Char);
                return Expr {
                    kind: ExprKind::Compare(op, CmpKind::Char, Box::new(left), Box::new(right)),
                    ty: Boolean,
                    range: at,
                };
            }
            (Boolean, _) | (_, Boolean) if ordering => {
                self.error(
                    at,
                    "a Boolean cannot be compared with `<`, `>`, `<=` or `>=`: those compare \
                     numbers, and a Boolean does not convert to one",
                );
                return error_expr(at);
            }
            // An `Object` next to a String is parsed as a number too
            // (recorded: `string_less_object`).
            (l, r)
                if ordering
                    && (l.is_numeric() || *l == String)
                    && (r.is_numeric() || *r == String) =>
            {
                (CmpKind::Numeric, Double)
            }
            (String, Object) | (Object, String) if ordering => (CmpKind::Numeric, Double),
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
