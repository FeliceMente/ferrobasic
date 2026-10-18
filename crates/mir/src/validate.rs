//! Structural checks on a [`Program`]: every id in range, every call with
//! the right number of arguments, Boolean branch conditions, a well-formed
//! entry point. The lowering must only produce programs that pass, and the
//! interpreter may assume they do.

use crate::*;

/// Returns the first problem found, as `function: message`.
pub fn validate(program: &Program) -> Result<(), String> {
    let funcs = program.functions.len();
    let check_func = |id: FuncId, what: &str| -> Result<(), String> {
        if (id.0 as usize) < funcs {
            Ok(())
        } else {
            Err(format!("{what} refers to missing function {}", id.0))
        }
    };
    check_func(program.init, "init")?;
    check_func(program.entry, "entry")?;
    let init = program.function(program.init);
    if init.params != 0 || init.ret != Ty::Void {
        return Err("init must take no parameters and return nothing".into());
    }
    let entry = program.function(program.entry);
    if entry.params != 1
        || entry.locals.first().map(|l| &l.ty) != Some(&Ty::Array(Box::new(Ty::String), 1))
    {
        return Err("entry must take one `String()` parameter".into());
    }
    for (i, record) in program.records.iter().enumerate() {
        check_func(record.init, &format!("record {}", record.name))?;
        let init = program.function(record.init);
        if init.params != 1 || init.locals[0].ty != Ty::Record(RecordId(i as u32)) {
            return Err(format!(
                "record {}: init must take the record as its only parameter",
                record.name
            ));
        }
    }
    for function in &program.functions {
        Checker { program, function }
            .check()
            .map_err(|e| format!("{}: {e}", function.name))?;
    }
    Ok(())
}

struct Checker<'a> {
    program: &'a Program,
    function: &'a Function,
}

impl Checker<'_> {
    fn check(&self) -> Result<(), String> {
        let f = self.function;
        if f.blocks.is_empty() {
            return Err("no blocks".into());
        }
        if f.params as usize > f.locals.len() {
            return Err("more parameters than locals".into());
        }
        for (i, block) in f.blocks.iter().enumerate() {
            let at = |e: String| format!("block {i}: {e}");
            if let Some(h) = block.handler {
                self.block_id(h).map_err(at)?;
            }
            for stmt in &block.stmts {
                self.statement(&stmt.kind).map_err(at)?;
            }
            match &block.terminator {
                Terminator::Goto(b) => self.block_id(*b).map_err(at)?,
                Terminator::Branch {
                    cond,
                    then,
                    otherwise,
                } => {
                    self.block_id(*then).map_err(at)?;
                    self.block_id(*otherwise).map_err(at)?;
                    self.operand(cond).map_err(at)?;
                    if let Operand::Local(l) = cond
                        && f.local(*l).ty != Ty::Boolean
                    {
                        return Err(at("branch condition is not a Boolean".into()));
                    }
                }
                Terminator::Return(value) => {
                    if let Some(v) = value {
                        if f.ret == Ty::Void {
                            return Err(at("returns a value from a Void function".into()));
                        }
                        self.operand(v).map_err(at)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn block_id(&self, id: BlockId) -> Result<(), String> {
        if (id.0 as usize) < self.function.blocks.len() {
            Ok(())
        } else {
            Err(format!("jump to missing block {}", id.0))
        }
    }

    fn local(&self, id: LocalId) -> Result<(), String> {
        if (id.0 as usize) < self.function.locals.len() {
            Ok(())
        } else {
            Err(format!("missing local {}", id.0))
        }
    }

    fn global(&self, id: GlobalId) -> Result<(), String> {
        if (id.0 as usize) < self.program.globals.len() {
            Ok(())
        } else {
            Err(format!("missing global {}", id.0))
        }
    }

    fn operand(&self, op: &Operand) -> Result<(), String> {
        match op {
            Operand::Local(l) => self.local(*l),
            Operand::Const(_) => Ok(()),
        }
    }

    fn operands<'o>(&self, ops: impl IntoIterator<Item = &'o Operand>) -> Result<(), String> {
        ops.into_iter().try_for_each(|o| self.operand(o))
    }

    fn statement(&self, stmt: &StatementKind) -> Result<(), String> {
        match stmt {
            StatementKind::Assign { dest, value } => {
                self.local(*dest)?;
                self.rvalue(value)
            }
            StatementKind::SetGlobal { global, value } => {
                self.global(*global)?;
                self.operand(value)
            }
            StatementKind::SetIndex {
                array,
                indices,
                value,
            } => {
                self.operand(array)?;
                self.operands(indices)?;
                self.operand(value)
            }
            StatementKind::SetField { record, value, .. } => {
                self.operand(record)?;
                self.operand(value)
            }
            StatementKind::Eval(value) => self.rvalue(value),
        }
    }

    fn rvalue(&self, value: &Rvalue) -> Result<(), String> {
        match value {
            Rvalue::Use(o)
            | Rvalue::Convert { value: o, .. }
            | Rvalue::Neg { value: o, .. }
            | Rvalue::Not(o)
            | Rvalue::Format { value: o, .. }
            | Rvalue::IterSize(o)
            | Rvalue::Field { record: o, .. } => self.operand(o),
            Rvalue::Global(g) => self.global(*g),
            Rvalue::Default(_) => Ok(()),
            Rvalue::Arith { left, right, .. } | Rvalue::Compare { left, right, .. } => {
                self.operand(left)?;
                self.operand(right)
            }
            Rvalue::Concat(parts) => self.operands(parts),
            Rvalue::Call { func, args } => {
                let Some(callee) = self.program.functions.get(func.0 as usize) else {
                    return Err(format!("call to missing function {}", func.0));
                };
                if callee.params as usize != args.len() {
                    return Err(format!(
                        "call to {} with {} argument(s), expected {}",
                        callee.name,
                        args.len(),
                        callee.params
                    ));
                }
                self.operands(args)
            }
            Rvalue::Builtin {
                builtin,
                receiver,
                args,
            } => {
                let sig = builtin.signature();
                if sig.params.len() != args.len() {
                    return Err(format!("{} with {} argument(s)", sig.name, args.len()));
                }
                if let Some(r) = receiver {
                    self.operand(r)?;
                }
                self.operands(args)
            }
            Rvalue::Index { array, indices } => {
                self.operand(array)?;
                self.operands(indices)
            }
            Rvalue::NewArray { dims, .. } => self.operands(dims),
            Rvalue::ArrayLiteral { items, .. } => self.operands(items),
            Rvalue::CreateMap(pairs) => pairs.iter().try_for_each(|(k, v)| {
                self.operand(k)?;
                self.operand(v)
            }),
            Rvalue::IterGet { collection, index } => {
                self.operand(collection)?;
                self.operand(index)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(name: &str, params: u32, locals: Vec<Local>, blocks: Vec<BasicBlock>) -> Function {
        Function {
            name: name.into(),
            params,
            locals,
            ret: Ty::Void,
            blocks,
        }
    }

    fn block(stmts: Vec<StatementKind>, terminator: Terminator) -> BasicBlock {
        BasicBlock {
            stmts: stmts
                .into_iter()
                .map(|kind| Statement {
                    kind,
                    range: TextRange::default(),
                })
                .collect(),
            terminator,
            handler: None,
        }
    }

    /// `Sub AppStart (Args() As String) : Log("hi") : End Sub`.
    fn hello() -> Program {
        let args = Local {
            name: Some("Args".into()),
            ty: Ty::Array(Box::new(Ty::String), 1),
        };
        let log = StatementKind::Eval(Rvalue::Builtin {
            builtin: Builtin::Log,
            receiver: None,
            args: vec![Operand::Const(Constant::String("hi".into()))],
        });
        Program {
            records: vec![],
            globals: vec![],
            functions: vec![
                function(
                    "Process_Globals",
                    0,
                    vec![],
                    vec![block(vec![], Terminator::Return(None))],
                ),
                function(
                    "AppStart",
                    1,
                    vec![args],
                    vec![block(vec![log], Terminator::Return(None))],
                ),
            ],
            init: FuncId(0),
            entry: FuncId(1),
        }
    }

    #[test]
    fn hello_world_is_valid() {
        validate(&hello()).unwrap();
    }

    #[test]
    fn bad_programs_are_rejected() {
        let mut p = hello();
        p.functions[1].blocks[0].terminator = Terminator::Goto(BlockId(7));
        assert!(validate(&p).unwrap_err().contains("missing block 7"));

        let mut p = hello();
        p.functions[1].blocks[0].stmts.push(Statement {
            kind: StatementKind::Eval(Rvalue::Call {
                func: FuncId(1),
                args: vec![],
            }),
            range: TextRange::default(),
        });
        assert!(
            validate(&p)
                .unwrap_err()
                .contains("0 argument(s), expected 1")
        );

        let mut p = hello();
        p.functions[1].locals[0].ty = Ty::String;
        assert!(validate(&p).unwrap_err().contains("entry must take"));
    }
}
