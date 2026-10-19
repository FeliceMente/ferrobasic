//! HIR → MIR lowering.
//!
//! Each HIR Sub becomes one MIR function with the same index; then come the
//! synthetic `Process_Globals` (only when the module has none) and one
//! `<Type>.Initialize` function per record. Globals and Sub locals keep their
//! HIR indices; temporaries are appended after the Sub's locals.
//!
//! Expressions become three-address code: every intermediate value is an
//! [`Operand`], either a constant or a local. Control-flow expressions
//! (`And`, `Or`, `IIf`) and statements (`If`, loops, `Select`, `Try`) become
//! blocks; see the doc comments on the `Builder` methods for each shape.

use ferro_hir::hir::{
    self as h, DimInit, ExprKind, Literal, LogicOp, Place, SmartPart, StmtKind, VarRef,
};

use crate::*;

/// Lowers a type-checked module, one `ferro_hir::check` returned without
/// diagnostics, to a program that passes [`crate::validate`].
///
/// # Panics
///
/// If the module has no `AppStart`: such a module always has diagnostics.
pub fn lower(module: &ferro_hir::Module) -> Program {
    let subs = module.subs.len() as u32;
    let mut functions: Vec<Function> = module
        .subs
        .iter()
        .map(|sub| lower_sub(module, sub))
        .collect();

    let init = match module.process_globals {
        Some(id) => FuncId(id.0),
        None => {
            functions.push(Function {
                name: "Process_Globals".into(),
                params: 0,
                locals: Vec::new(),
                ret: Ty::Void,
                blocks: vec![BasicBlock {
                    stmts: Vec::new(),
                    terminator: Terminator::Return(None),
                    handler: None,
                }],
            });
            FuncId(subs)
        }
    };
    let entry = FuncId(
        module
            .app_start
            .expect("a module without `AppStart` has diagnostics")
            .0,
    );

    let mut records = Vec::with_capacity(module.records.len());
    for (i, record) in module.records.iter().enumerate() {
        let id = FuncId(functions.len() as u32);
        functions.push(lower_record_init(module, RecordId(i as u32), record));
        records.push(Record {
            name: record.name.clone(),
            fields: record
                .fields
                .iter()
                .map(|f| RecordField {
                    name: f.name.clone(),
                    ty: f.ty.clone(),
                })
                .collect(),
            init: id,
        });
    }

    Program {
        records,
        globals: module
            .globals
            .iter()
            .map(|g| Global {
                name: g.name.clone(),
                ty: g.ty.clone(),
            })
            .collect(),
        functions,
        init,
        entry,
    }
}

fn lower_sub(module: &h::Module, sub: &h::Sub) -> Function {
    let locals = sub
        .locals
        .iter()
        .map(|v| Local {
            name: Some(v.name.clone()),
            ty: v.ty.clone(),
        })
        .collect();
    let mut b = Builder::new(module, locals, sub.range);
    b.block(&sub.body);
    b.finish(sub.name.clone(), sub.params, sub.ret.clone())
}

/// `<Type>.Initialize`: assigns every field its default value, or a new
/// array for fields declared with bounds.
fn lower_record_init(module: &h::Module, id: RecordId, record: &h::Record) -> Function {
    let locals = vec![Local {
        name: Some("record".into()),
        ty: Ty::Record(id),
    }];
    let mut b = Builder::new(module, locals, record.range);
    let this = Operand::Local(LocalId(0));
    for (i, field) in record.fields.iter().enumerate() {
        let value = match &field.ty {
            Ty::Array(elem, _) if !field.bounds.is_empty() => {
                let dims = field.bounds.iter().map(|e| b.operand(e)).collect();
                Rvalue::NewArray {
                    elem: (**elem).clone(),
                    dims,
                }
            }
            ty => Rvalue::Default(ty.clone()),
        };
        let value = b.as_operand(value, &field.ty);
        b.push(StatementKind::SetField {
            record: this.clone(),
            field: i as u32,
            value,
        });
    }
    b.finish(format!("{}.Initialize", record.name), 1, Ty::Void)
}

/// What `Exit` and `Continue` can jump to.
enum Scope {
    Loop { exit: BlockId, next: BlockId },
    Select { exit: BlockId },
}

struct Builder<'m> {
    module: &'m h::Module,
    locals: Vec<Local>,
    blocks: Vec<BasicBlock>,
    current: BlockId,
    /// The handler given to blocks created now (the innermost `Try`'s catch).
    handler: Option<BlockId>,
    scopes: Vec<Scope>,
    /// The range of the HIR statement being lowered.
    range: TextRange,
}

impl<'m> Builder<'m> {
    fn new(module: &'m h::Module, locals: Vec<Local>, range: TextRange) -> Self {
        let mut b = Builder {
            module,
            locals,
            blocks: Vec::new(),
            current: BlockId(0),
            handler: None,
            scopes: Vec::new(),
            range,
        };
        b.current = b.new_block();
        b
    }

    /// Falls through to `Return(None)` at the end of the body.
    fn finish(mut self, name: String, params: u32, ret: Ty) -> Function {
        let last = self.current;
        self.terminate_into(Terminator::Return(None), last);
        Function {
            name,
            params,
            locals: self.locals,
            ret,
            blocks: self.blocks,
        }
    }

    // ----- Building blocks ---------------------------------------------------

    /// A new block with the current handler; its terminator is a placeholder
    /// until it is terminated.
    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(BasicBlock {
            stmts: Vec::new(),
            terminator: Terminator::Return(None),
            handler: self.handler,
        });
        id
    }

    fn switch_to(&mut self, block: BlockId) {
        self.current = block;
    }

    /// Ends the current block. Code that follows (after `Return`, `Exit`...)
    /// goes to a fresh, unreachable block.
    fn terminate(&mut self, terminator: Terminator) {
        self.blocks[self.current.0 as usize].terminator = terminator;
        self.current = self.new_block();
    }

    /// Ends the current block with `terminator` and continues in `next`.
    fn terminate_into(&mut self, terminator: Terminator, next: BlockId) {
        self.blocks[self.current.0 as usize].terminator = terminator;
        self.current = next;
    }

    fn goto(&mut self, target: BlockId, next: BlockId) {
        self.terminate_into(Terminator::Goto(target), next);
    }

    fn branch(&mut self, cond: Operand, then: BlockId, otherwise: BlockId, next: BlockId) {
        self.terminate_into(
            Terminator::Branch {
                cond,
                then,
                otherwise,
            },
            next,
        );
    }

    fn push(&mut self, kind: StatementKind) {
        let range = self.range;
        self.blocks[self.current.0 as usize]
            .stmts
            .push(Statement { kind, range });
    }

    fn temp(&mut self, ty: Ty) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(Local { name: None, ty });
        id
    }

    /// Stores `value` into a new temporary of type `ty`.
    fn assign_temp(&mut self, value: Rvalue, ty: &Ty) -> LocalId {
        let dest = self.temp(ty.clone());
        self.push(StatementKind::Assign { dest, value });
        dest
    }

    /// `value` as an operand: itself for `Use`, else a new temporary.
    fn as_operand(&mut self, value: Rvalue, ty: &Ty) -> Operand {
        match value {
            Rvalue::Use(op) => op,
            value => Operand::Local(self.assign_temp(value, ty)),
        }
    }

    /// `op`, copied to a fresh temporary when it names a variable, so that
    /// later assignments to the variable do not change it. Temporaries are
    /// never reassigned once their expression is computed.
    fn snapshot(&mut self, op: Operand, ty: &Ty) -> Operand {
        match op {
            Operand::Local(l) if self.locals[l.0 as usize].name.is_some() => {
                Operand::Local(self.assign_temp(Rvalue::Use(op), ty))
            }
            op => op,
        }
    }

    fn var_ty(&self, var: VarRef) -> Ty {
        match var {
            VarRef::Global(g) => self.module.global(g).ty.clone(),
            VarRef::Local(l) => self.locals[l.0 as usize].ty.clone(),
        }
    }

    fn read_var(&mut self, var: VarRef) -> Operand {
        match var {
            VarRef::Local(l) => Operand::Local(LocalId(l.0)),
            VarRef::Global(g) => {
                let ty = self.var_ty(var);
                Operand::Local(self.assign_temp(Rvalue::Global(GlobalId(g.0)), &ty))
            }
        }
    }

    fn write_var(&mut self, var: VarRef, value: Rvalue) {
        match var {
            VarRef::Local(l) => self.push(StatementKind::Assign {
                dest: LocalId(l.0),
                value,
            }),
            VarRef::Global(g) => {
                let ty = self.var_ty(var);
                let value = self.as_operand(value, &ty);
                self.push(StatementKind::SetGlobal {
                    global: GlobalId(g.0),
                    value,
                });
            }
        }
    }

    /// `value` (of type `from`) as a Double operand, for numeric comparisons.
    fn as_double(&mut self, value: Operand, from: &Ty) -> Operand {
        if *from == Ty::Double {
            value
        } else {
            let convert = Rvalue::Convert {
                value,
                from: from.clone(),
                to: Ty::Double,
            };
            Operand::Local(self.assign_temp(convert, &Ty::Double))
        }
    }

    fn compare(&mut self, op: CmpOp, kind: CmpKind, left: Operand, right: Operand) -> Operand {
        let value = Rvalue::Compare {
            op,
            kind,
            left,
            right,
        };
        Operand::Local(self.assign_temp(value, &Ty::Boolean))
    }

    // ----- Statements --------------------------------------------------------

    fn block(&mut self, block: &h::Block) {
        for stmt in block {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &h::Stmt) {
        let outer = std::mem::replace(&mut self.range, stmt.range);
        match &stmt.kind {
            StmtKind::Dim { var, init } => {
                let ty = self.var_ty(*var);
                let value = match init {
                    DimInit::Default => Rvalue::Default(ty),
                    DimInit::Value(e) => self.rvalue(e),
                    DimInit::Array(bounds) => {
                        let elem = match &ty {
                            Ty::Array(elem, _) => (**elem).clone(),
                            other => other.clone(),
                        };
                        let dims = if bounds.is_empty() {
                            vec![Operand::Const(Constant::Int(0))]
                        } else {
                            bounds.iter().map(|e| self.operand(e)).collect()
                        };
                        Rvalue::NewArray { elem, dims }
                    }
                };
                self.write_var(*var, value);
            }
            StmtKind::Assign { target, value } => self.assign(target, value),
            StmtKind::Expr(e) => {
                // Anything but a call computes a value without effects; its
                // sub-expressions (and their effects) are already emitted.
                let value = self.rvalue(e);
                if value.has_effects() {
                    self.push(StatementKind::Eval(value));
                }
            }
            StmtKind::If {
                branches,
                else_block,
            } => self.if_chain(branches, else_block.as_ref()),
            StmtKind::Select {
                scrutinee,
                kind,
                cases,
                else_block,
            } => self.select(scrutinee, *kind, cases, else_block.as_ref()),
            StmtKind::For {
                var,
                start,
                end,
                step,
                body,
            } => self.for_loop(*var, start, end, step.as_ref(), body),
            StmtKind::ForEach {
                var,
                collection,
                element,
                body,
            } => self.for_each(*var, collection, element, body),
            StmtKind::Do { cond, until, body } => self.do_loop(cond, *until, body),
            StmtKind::Try { body, catch } => self.try_catch(body, catch),
            StmtKind::Return(value) => {
                let value = value.as_ref().map(|e| self.operand(e));
                self.terminate(Terminator::Return(value));
            }
            StmtKind::Exit => {
                let target = match self.scopes.last() {
                    Some(Scope::Loop { exit, .. } | Scope::Select { exit }) => *exit,
                    None => unreachable!("`Exit` outside a loop or `Select` has a diagnostic"),
                };
                self.terminate(Terminator::Goto(target));
            }
            StmtKind::Continue => {
                let target = self
                    .scopes
                    .iter()
                    .rev()
                    .find_map(|s| match s {
                        Scope::Loop { next, .. } => Some(*next),
                        Scope::Select { .. } => None,
                    })
                    .expect("`Continue` outside a loop has a diagnostic");
                self.terminate(Terminator::Goto(target));
            }
        }
        self.range = outer;
    }

    fn assign(&mut self, target: &Place, value: &h::Expr) {
        match target {
            Place::Var(var) => {
                let value = self.rvalue(value);
                self.write_var(*var, value);
            }
            Place::Index { array, indices } => {
                let (array, index) = self.select_row(array, indices);
                let value = self.operand(value);
                self.push(StatementKind::SetIndex {
                    array,
                    indices: vec![index],
                    value,
                });
            }
            Place::Field { record, field } => {
                let record = self.operand(record);
                let value = self.operand(value);
                self.push(StatementKind::SetField {
                    record,
                    field: *field,
                    value,
                });
            }
        }
    }

    /// Each condition is tested in turn; the first true one runs its block.
    fn if_chain(&mut self, branches: &[(h::Expr, h::Block)], else_block: Option<&h::Block>) {
        let end = self.new_block();
        for (cond, body) in branches {
            let cond = self.operand(cond);
            let then = self.new_block();
            let next = self.new_block();
            self.branch(cond, then, next, then);
            self.block(body);
            self.goto(end, next);
        }
        if let Some(body) = else_block {
            self.block(body);
        }
        self.goto(end, end);
    }

    /// The scrutinee is evaluated once; case values are evaluated and
    /// compared in order until one matches. No fall-through; `Exit` leaves.
    fn select(
        &mut self,
        scrutinee: &h::Expr,
        kind: CmpKind,
        cases: &[h::Case],
        else_block: Option<&h::Block>,
    ) {
        let value = self.operand(scrutinee);
        let value = self.snapshot(value, &scrutinee.ty);
        let end = self.new_block();
        self.scopes.push(Scope::Select { exit: end });
        // The bodies are lowered after all tests, so blocks stay in test order.
        let mut bodies = Vec::with_capacity(cases.len());
        for case in cases {
            let body = self.new_block();
            for v in &case.values {
                let v = self.operand(v);
                let matched = self.compare(CmpOp::Eq, kind, value.clone(), v);
                let next = self.new_block();
                self.branch(matched, body, next, next);
            }
            bodies.push((body, &case.body));
        }
        if let Some(body) = else_block {
            self.block(body);
        }
        self.goto(end, end);
        for (block, body) in bodies {
            self.switch_to(block);
            self.block(body);
            self.goto(end, end);
        }
        self.scopes.pop();
        self.switch_to(end);
    }

    /// ```text
    ///     var = start; limit = end; step = step
    /// cond:   test (by the sign of step) -> body | exit
    /// body:   ...; goto next
    /// next:   var = var + step; goto cond
    /// exit:
    /// ```
    fn for_loop(
        &mut self,
        var: VarRef,
        start: &h::Expr,
        end: &h::Expr,
        step: Option<&h::Expr>,
        body: &h::Block,
    ) {
        let ty = self.var_ty(var);
        let start = self.rvalue(start);
        self.write_var(var, start);
        // The limit is only used compared as a Double.
        let limit = self.operand(end);
        let limit_d = if ty == Ty::Double {
            self.snapshot(limit, &ty)
        } else {
            self.as_double(limit, &ty)
        };
        let (step, sign) = match step {
            Some(e) => {
                let sign = constant_sign(e);
                let op = self.operand(e);
                (self.snapshot(op, &ty), sign)
            }
            None => (self.one(&ty), Some(Sign::Positive)),
        };
        // A step whose sign is only known at run time is tested as a Double.
        let step_d = match sign {
            Some(sign) => Err(sign),
            None => Ok(self.as_double(step.clone(), &ty)),
        };

        let cond = self.new_block();
        let body_block = self.new_block();
        let next = self.new_block();
        let exit = self.new_block();
        self.goto(cond, cond);

        let current = self.read_var(var);
        let current = self.as_double(current, &ty);
        match step_d {
            Err(sign) => {
                let op = match sign {
                    Sign::Positive => CmpOp::Le,
                    Sign::Negative => CmpOp::Ge,
                };
                let test = self.compare(op, CmpKind::Numeric, current, limit_d);
                self.branch(test, body_block, exit, body_block);
            }
            Ok(step_d) => {
                // (step > 0 And var <= limit) Or (step < 0 And var >= limit).
                let zero = Operand::Const(Constant::Double(0.0));
                let positive =
                    self.compare(CmpOp::Gt, CmpKind::Numeric, step_d.clone(), zero.clone());
                let up = self.new_block();
                let not_up = self.new_block();
                self.branch(positive, up, not_up, up);
                let test = self.compare(
                    CmpOp::Le,
                    CmpKind::Numeric,
                    current.clone(),
                    limit_d.clone(),
                );
                self.branch(test, body_block, exit, not_up);
                let negative = self.compare(CmpOp::Lt, CmpKind::Numeric, step_d, zero);
                let down = self.new_block();
                self.branch(negative, down, exit, down);
                let test = self.compare(CmpOp::Ge, CmpKind::Numeric, current, limit_d);
                self.branch(test, body_block, exit, body_block);
            }
        }

        self.scopes.push(Scope::Loop { exit, next });
        self.block(body);
        self.scopes.pop();
        self.goto(next, next);

        let current = self.read_var(var);
        let sum = Rvalue::Arith {
            op: ArithOp::Add,
            ty: ty.clone(),
            left: current,
            right: step,
        };
        self.write_var(var, sum);
        self.goto(cond, exit);
    }

    /// The constant 1 of a numeric type, as an operand.
    fn one(&mut self, ty: &Ty) -> Operand {
        match ty {
            Ty::Int => Operand::Const(Constant::Int(1)),
            Ty::Long => Operand::Const(Constant::Long(1)),
            Ty::Double => Operand::Const(Constant::Double(1.0)),
            ty => {
                let convert = Rvalue::Convert {
                    value: Operand::Const(Constant::Int(1)),
                    from: Ty::Int,
                    to: ty.clone(),
                };
                Operand::Local(self.assign_temp(convert, ty))
            }
        }
    }

    /// ```text
    ///     coll = collection; size = IterSize(coll); index = 0
    /// cond:   index < size -> body | exit
    /// body:   var = IterGet(coll, index) (converted); ...; goto next
    /// next:   index = index + 1; goto cond
    /// exit:
    /// ```
    fn for_each(&mut self, var: VarRef, collection: &h::Expr, element: &Ty, body: &h::Block) {
        let ty = self.var_ty(var);
        let coll = self.operand(collection);
        let coll = self.snapshot(coll, &collection.ty);
        let size = self.assign_temp(Rvalue::IterSize(coll.clone()), &Ty::Int);
        let size_d = self.as_double(Operand::Local(size), &Ty::Int);
        let index = self.assign_temp(Rvalue::Use(Operand::Const(Constant::Int(0))), &Ty::Int);

        let cond = self.new_block();
        let body_block = self.new_block();
        let next = self.new_block();
        let exit = self.new_block();
        self.goto(cond, cond);

        let index_d = self.as_double(Operand::Local(index), &Ty::Int);
        let test = self.compare(CmpOp::Lt, CmpKind::Numeric, index_d, size_d);
        self.branch(test, body_block, exit, body_block);

        let get = Rvalue::IterGet {
            collection: coll,
            index: Operand::Local(index),
        };
        let item = self.assign_temp(get, element);
        let value = if *element == ty {
            Rvalue::Use(Operand::Local(item))
        } else {
            Rvalue::Convert {
                value: Operand::Local(item),
                from: element.clone(),
                to: ty,
            }
        };
        self.write_var(var, value);
        self.scopes.push(Scope::Loop { exit, next });
        self.block(body);
        self.scopes.pop();
        self.goto(next, next);

        self.push(StatementKind::Assign {
            dest: index,
            value: Rvalue::Arith {
                op: ArithOp::Add,
                ty: Ty::Int,
                left: Operand::Local(index),
                right: Operand::Const(Constant::Int(1)),
            },
        });
        self.goto(cond, exit);
    }

    /// The condition is tested before each iteration; `Continue` re-tests it.
    fn do_loop(&mut self, cond: &h::Expr, until: bool, body: &h::Block) {
        let head = self.new_block();
        let body_block = self.new_block();
        let exit = self.new_block();
        self.goto(head, head);
        let test = self.operand(cond);
        let (then, otherwise) = if until {
            (exit, body_block)
        } else {
            (body_block, exit)
        };
        self.branch(test, then, otherwise, body_block);
        self.scopes.push(Scope::Loop { exit, next: head });
        self.block(body);
        self.scopes.pop();
        self.goto(head, exit);
    }

    /// Every block of the Try body has the catch block as handler; the catch
    /// blocks and the join keep the enclosing handler.
    fn try_catch(&mut self, body: &h::Block, catch: &h::Block) {
        let catch_block = self.new_block();
        let end = self.new_block();
        let outer = self.handler.replace(catch_block);
        let body_block = self.new_block();
        self.goto(body_block, body_block);
        self.block(body);
        self.handler = outer;
        // `catch_block` and `end` were created before the handler was set,
        // so they keep the enclosing one.
        self.goto(end, catch_block);
        self.block(catch);
        self.goto(end, end);
    }

    // ----- Expressions -------------------------------------------------------

    /// Lowers `e` to an operand, emitting the code that computes it.
    /// `a(i, j, k)` as Java evaluates `a[i][j][k]`: each index selects a
    /// row (checked, and kept in a temporary) before the next index is
    /// evaluated, as rows can be replaced. Returns the innermost row and
    /// the last index.
    fn select_row(&mut self, array: &h::Expr, indices: &[h::Expr]) -> (Operand, Operand) {
        let mut row = self.operand(array);
        let mut ty = array.ty.clone();
        let (last, rows) = indices
            .split_last()
            .expect("an index expression has an index");
        for index in rows {
            let index = self.operand(index);
            ty = match ty {
                Ty::Array(elem, rank) if rank > 1 => Ty::Array(elem, rank - 1),
                other => other,
            };
            let value = Rvalue::Index {
                array: row,
                indices: vec![index],
            };
            row = Operand::Local(self.assign_temp(value, &ty));
        }
        (row, self.operand(last))
    }

    fn operand(&mut self, e: &h::Expr) -> Operand {
        let value = self.rvalue(e);
        self.as_operand(value, &e.ty)
    }

    /// Lowers `e` to an rvalue whose operands are already computed.
    fn rvalue(&mut self, e: &h::Expr) -> Rvalue {
        match &e.kind {
            ExprKind::Literal(lit) => Rvalue::Use(Operand::Const(constant(lit))),
            ExprKind::Var(VarRef::Local(l)) => Rvalue::Use(Operand::Local(LocalId(l.0))),
            ExprKind::Var(VarRef::Global(g)) => Rvalue::Global(GlobalId(g.0)),
            ExprKind::Convert(inner) => {
                if inner.ty == Ty::Null {
                    // A typed `Null`: the constant takes the type it is used as.
                    return Rvalue::Use(Operand::Const(Constant::Null));
                }
                let value = self.operand(inner);
                Rvalue::Convert {
                    value,
                    from: inner.ty.clone(),
                    to: e.ty.clone(),
                }
            }
            ExprKind::Neg(inner) => Rvalue::Neg {
                value: self.operand(inner),
                ty: e.ty.clone(),
            },
            ExprKind::Arith(op, l, r) => {
                let left = self.operand(l);
                let right = self.operand(r);
                Rvalue::Arith {
                    op: *op,
                    ty: e.ty.clone(),
                    left,
                    right,
                }
            }
            ExprKind::Concat(..) => {
                let mut parts = Vec::new();
                self.concat_parts(e, &mut parts);
                Rvalue::Concat(parts)
            }
            ExprKind::Compare(op, kind, l, r) => {
                let left = self.operand(l);
                let right = self.operand(r);
                Rvalue::Compare {
                    op: *op,
                    kind: *kind,
                    left,
                    right,
                }
            }
            ExprKind::Logic(op, l, r) => Rvalue::Use(self.logic(*op, l, r)),
            ExprKind::Call { sub, args } => Rvalue::Call {
                func: FuncId(sub.0),
                args: args.iter().map(|a| self.operand(a)).collect(),
            },
            ExprKind::Builtin {
                builtin,
                receiver,
                args,
            } => {
                let receiver = receiver.as_ref().map(|r| self.operand(r));
                let args = args.iter().map(|a| self.operand(a)).collect();
                Rvalue::Builtin {
                    builtin: *builtin,
                    receiver,
                    args,
                }
            }
            ExprKind::Index { array, indices } => {
                let (array, index) = self.select_row(array, indices);
                Rvalue::Index {
                    array,
                    indices: vec![index],
                }
            }
            ExprKind::Field { record, field } => Rvalue::Field {
                record: self.operand(record),
                field: *field,
            },
            ExprKind::SmartString(parts) => {
                let parts = parts
                    .iter()
                    .map(|part| match part {
                        SmartPart::Text(t) => Operand::Const(Constant::String(t.clone())),
                        SmartPart::Value { expr, format } if format.is_empty() => {
                            self.operand(expr)
                        }
                        SmartPart::Value { expr, format } => {
                            let value = self.operand(expr);
                            let format = Rvalue::Format {
                                format: format.clone(),
                                value,
                            };
                            Operand::Local(self.assign_temp(format, &Ty::String))
                        }
                    })
                    .collect();
                Rvalue::Concat(parts)
            }
            ExprKind::ArrayLiteral(items) => {
                let elem = match &e.ty {
                    Ty::Array(elem, _) => (**elem).clone(),
                    _ => Ty::Object,
                };
                let items = items.iter().map(|i| self.operand(i)).collect();
                Rvalue::ArrayLiteral { elem, items }
            }
            ExprKind::CreateMap(pairs) => Rvalue::CreateMap(
                pairs
                    .iter()
                    .map(|(k, v)| {
                        let k = self.operand(k);
                        let v = self.operand(v);
                        (k, v)
                    })
                    .collect(),
            ),
            ExprKind::IIf(cond, a, b) => Rvalue::Use(self.iif(cond, a, b, &e.ty)),
            ExprKind::Error => Rvalue::Default(e.ty.clone()),
        }
    }

    /// Flattens `a & b & c` into one concatenation, left to right.
    fn concat_parts(&mut self, e: &h::Expr, parts: &mut Vec<Operand>) {
        match &e.kind {
            ExprKind::Concat(l, r) => {
                self.concat_parts(l, parts);
                self.concat_parts(r, parts);
            }
            _ => parts.push(self.operand(e)),
        }
    }

    /// ```text
    ///     l = left; branch l -> rhs | short   (And; swapped for Or)
    /// rhs:    result = right; goto end
    /// short:  result = False (And) / True (Or); goto end
    /// end:
    /// ```
    fn logic(&mut self, op: LogicOp, l: &h::Expr, r: &h::Expr) -> Operand {
        let result = self.temp(Ty::Boolean);
        let left = self.operand(l);
        let rhs = self.new_block();
        let short = self.new_block();
        let end = self.new_block();
        let (then, otherwise) = match op {
            LogicOp::And => (rhs, short),
            LogicOp::Or => (short, rhs),
        };
        self.branch(left, then, otherwise, rhs);
        let right = self.rvalue(r);
        self.push(StatementKind::Assign {
            dest: result,
            value: right,
        });
        self.goto(end, short);
        self.push(StatementKind::Assign {
            dest: result,
            value: Rvalue::Use(Operand::Const(Constant::Bool(op == LogicOp::Or))),
        });
        self.goto(end, end);
        Operand::Local(result)
    }

    /// Only the chosen value is evaluated.
    fn iif(&mut self, cond: &h::Expr, a: &h::Expr, b: &h::Expr, ty: &Ty) -> Operand {
        let result = self.temp(ty.clone());
        let cond = self.operand(cond);
        let then = self.new_block();
        let otherwise = self.new_block();
        let end = self.new_block();
        self.branch(cond, then, otherwise, then);
        for (value, next) in [(a, otherwise), (b, end)] {
            let value = self.rvalue(value);
            self.push(StatementKind::Assign {
                dest: result,
                value,
            });
            self.goto(end, next);
        }
        Operand::Local(result)
    }
}

fn constant(lit: &Literal) -> Constant {
    match lit {
        Literal::Bool(b) => Constant::Bool(*b),
        Literal::Int(i) => Constant::Int(*i),
        Literal::Long(i) => Constant::Long(*i),
        Literal::Double(d) => Constant::Double(*d),
        Literal::String(s) => Constant::String(s.clone()),
        Literal::Null => Constant::Null,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    Positive,
    Negative,
}

/// The sign of a constant `For` step, when it is a non-zero constant whose
/// sign does not depend on a conversion rule. A zero step, or any other
/// expression, is tested at run time.
fn constant_sign(e: &h::Expr) -> Option<Sign> {
    match constant_value(e)? {
        Folded::Int(v) if v > 0 => Some(Sign::Positive),
        Folded::Int(v) if v < 0 => Some(Sign::Negative),
        Folded::Double(v) if v > 0.0 => Some(Sign::Positive),
        Folded::Double(v) if v < 0.0 => Some(Sign::Negative),
        // Zero and NaN are decided at run time.
        _ => None,
    }
}

/// A folded step value, in the precision of the step's type.
enum Folded {
    /// An Int or Long value, exact.
    Int(i64),
    /// A Double value, exact.
    Double(f64),
}

/// The value of a constant step, only when folding it is provably exact:
/// integer literals with checked negation, widening to Long or Double, and
/// Double literals. Anything else (Float, Byte and Short steps, narrowing,
/// values that would wrap) is left to the run-time sign test.
fn constant_value(e: &h::Expr) -> Option<Folded> {
    match (&e.kind, &e.ty) {
        (ExprKind::Literal(Literal::Int(i)), Ty::Int) => Some(Folded::Int(i64::from(*i))),
        (ExprKind::Literal(Literal::Long(i)), Ty::Long) => Some(Folded::Int(*i)),
        (ExprKind::Literal(Literal::Double(d)), Ty::Double) => Some(Folded::Double(*d)),
        (ExprKind::Neg(inner), Ty::Int) => match constant_value(inner)? {
            Folded::Int(v) => i32::try_from(v.checked_neg()?)
                .ok()
                .map(|v| Folded::Int(v.into())),
            Folded::Double(_) => None,
        },
        (ExprKind::Neg(inner), Ty::Long) => match constant_value(inner)? {
            Folded::Int(v) => v.checked_neg().map(Folded::Int),
            Folded::Double(_) => None,
        },
        (ExprKind::Neg(inner), Ty::Double) => match constant_value(inner)? {
            Folded::Double(v) => Some(Folded::Double(-v)),
            Folded::Int(_) => None,
        },
        (ExprKind::Convert(inner), Ty::Long) if inner.ty == Ty::Int => constant_value(inner),
        (ExprKind::Convert(inner), Ty::Double) if inner.ty.is_integer() => {
            match constant_value(inner)? {
                // Exact only within the 53-bit mantissa.
                Folded::Int(v) if v.unsigned_abs() <= 1 << 53 => Some(Folded::Double(v as f64)),
                _ => None,
            }
        }
        _ => None,
    }
}
