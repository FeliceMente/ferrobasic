//! Stateful Level 0 submissions. Compiler transactions do not copy runtime
//! objects; successful compilation commits before execution starts.
use std::io::{BufRead, Write};

use ferro_hir::{Module, SessionBindings, Ty};
use ferro_syntax::{Completeness, FragmentKind, SyntaxKind, TextRange};

pub const HELP: &str = "Enter BASIC statements, expressions, Types or complete Subs.\n:help    Show this help\n:cancel  Discard pending input\n:reset   Clear the session\n:quit    Leave the REPL\n";

#[derive(Clone)]
struct Cell {
    id: usize,
    text: String,
}
#[derive(Clone)]
struct Definition {
    name: String,
    cell: Cell,
    signature: Option<(Vec<Ty>, Ty)>,
}
struct Span {
    start: usize,
    end: usize,
    cell: Cell,
}
#[derive(Default)]
struct Source {
    text: String,
    spans: Vec<Span>,
}
impl Source {
    fn push(&mut self, text: &str) {
        self.text.push_str(text);
    }
    fn cell(&mut self, cell: &Cell) {
        let start = self.text.len();
        self.push(&cell.text);
        self.spans.push(Span {
            start,
            end: self.text.len(),
            cell: cell.clone(),
        });
        if !self.text.ends_with('\n') {
            self.push("\n");
        }
    }
    fn diagnostic(&self, range: TextRange, message: &str) -> String {
        let offset = usize::from(range.start());
        let span = self
            .spans
            .iter()
            .find(|s| s.start <= offset && offset <= s.end)
            .or_else(|| self.spans.last());
        let Some(span) = span else {
            return format!("repl: error: {message}\n");
        };
        let offset = offset.saturating_sub(span.start).min(span.cell.text.len());
        let mut offset = offset;
        while !span.cell.text.is_char_boundary(offset) {
            offset -= 1;
        }
        let before = &span.cell.text[..offset];
        let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
        let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        format!(
            "<repl:{}>:{line}:{column}: error: {message}\n",
            span.cell.id
        )
    }
}

/// Persistent language bindings, definition sources and opaque interpreter state.
#[derive(Default)]
pub struct Session {
    bindings: SessionBindings,
    module: Module,
    types: Vec<Definition>,
    subs: Vec<Definition>,
    state: ferro_interp::SessionState,
    next_id: usize,
}

/// A completed submission: errors use CLI codes, explicit exit ends a session.
pub struct Submission {
    pub code: u8,
    pub exited: bool,
}

impl Session {
    /// Compiles and executes one complete submission. Input accumulation is
    /// separate, so tests and other front ends can share the session engine.
    pub fn submit(&mut self, text: &str, out: &mut dyn Write, err: &mut dyn Write) -> Submission {
        self.next_id += 1;
        let cell = Cell {
            id: self.next_id,
            text: text.to_owned(),
        };
        let (fragment, kind, status) = ferro_syntax::parse_fragment(text);
        let mut direct = Source::default();
        direct.cell(&cell);
        let reject = |message: &str, err: &mut dyn Write| {
            let _ = writeln!(err, "<repl:{}>:1:1: error: {message}", cell.id);
            Submission {
                code: 3,
                exited: false,
            }
        };
        if status != Completeness::Complete {
            for error in fragment.errors() {
                let _ = err.write_all(direct.diagnostic(error.range, &error.message).as_bytes());
            }
            if fragment.errors().is_empty() {
                return reject("incomplete input", err);
            }
            return Submission {
                code: 3,
                exited: false,
            };
        }
        let root = fragment.syntax();
        if root.descendants().any(|n| {
            matches!(
                n.kind(),
                SyntaxKind::ATTRIBUTE | SyntaxKind::DIRECTIVE_LINE | SyntaxKind::INLINE_JAVA_BLOCK
            )
        }) {
            return reject(
                "project attributes and directives are not accepted in a REPL",
                err,
            );
        }
        let definitions = root
            .children()
            .filter(|n| matches!(n.kind(), SyntaxKind::SUB_DECL | SyntaxKind::TYPE_DECL))
            .count();
        if definitions > 0 && !matches!(kind, FragmentKind::Sub | FragmentKind::Type) {
            return reject("submit each Sub or Type definition separately", err);
        }
        let mut types = self.types.clone();
        let mut subs = self.subs.clone();
        let name = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::NAME)
            .map(|n| n.text().to_string().trim().to_ascii_lowercase());
        if kind == FragmentKind::Sub {
            let name = name.as_deref().unwrap_or("");
            if ["appstart", "process_globals", "class_globals"].contains(&name) {
                return reject("the session supplies its own module structure", err);
            }
            if !subs.iter().any(|d| d.name == name)
                && self
                    .module
                    .globals
                    .iter()
                    .any(|v| v.name.eq_ignore_ascii_case(name))
            {
                return reject(
                    "a new Sub cannot change an existing session variable's binding; use another name",
                    err,
                );
            }
            if !subs.iter().any(|d| d.name == name)
                && self.module.subs.iter().any(|sub| {
                    sub.locals
                        .iter()
                        .skip(sub.params as usize)
                        .any(|v| v.name.eq_ignore_ascii_case(name))
                })
            {
                return reject(
                    "a new Sub would change an existing local binding; use another name",
                    err,
                );
            }
            let definition = Definition {
                name: name.into(),
                cell: cell.clone(),
                signature: None,
            };
            if let Some(old) = subs.iter_mut().find(|d| d.name == name) {
                *old = definition;
            } else {
                subs.push(definition);
            }
        } else if kind == FragmentKind::Type {
            let name = name.as_deref().unwrap_or("");
            if types.iter().any(|d| d.name == name) {
                return reject(
                    "a Type cannot be redeclared; use :reset to change its layout",
                    err,
                );
            }
            types.push(Definition {
                name: name.into(),
                cell: cell.clone(),
                signature: None,
            });
        } else {
            // A new Const cannot change the mutability of an existing slot;
            // a declaration cannot reset a previously accepted constant.
            let mut declared: Vec<_> = self
                .module
                .globals
                .iter()
                .map(|v| (v.name.to_ascii_lowercase(), v.is_const))
                .collect();
            for decl in root
                .descendants()
                .filter(|n| n.kind() == SyntaxKind::DECLARATOR)
            {
                let name = decl
                    .children()
                    .find(|n| n.kind() == SyntaxKind::NAME)
                    .map(|n| n.text().to_string().trim().to_owned())
                    .unwrap_or_default();
                let constant = decl
                    .parent()
                    .is_some_and(|p| p.children().any(|n| n.kind() == SyntaxKind::CONST_MODIFIER));
                if declared
                    .iter()
                    .any(|(n, is_const)| n.eq_ignore_ascii_case(&name) && (constant || *is_const))
                {
                    return reject("constants cannot be redefined; use :reset", err);
                }
                declared.push((name.to_ascii_lowercase(), constant));
            }
        }
        let build = |echo: bool| {
            let mut source = Source::default();
            source.push("Sub Process_Globals\n");
            for definition in &types {
                source.cell(&definition.cell);
            }
            if !matches!(kind, FragmentKind::Sub | FragmentKind::Type) {
                if echo {
                    let (tokens, _) = ferro_syntax::lexer::lex(&cell.text);
                    let mut offset = 0usize;
                    let mut end = 0usize;
                    for token in tokens {
                        offset += token.len as usize;
                        if !token.kind.is_trivia() && token.kind != SyntaxKind::NEWLINE {
                            end = offset;
                        }
                    }
                    source.push("Log(");
                    let start = source.text.len();
                    source.push(&cell.text[..end]);
                    source.spans.push(Span {
                        start,
                        end: source.text.len(),
                        cell: cell.clone(),
                    });
                    source.push(")\n");
                } else {
                    source.cell(&cell);
                }
            }
            source.push("End Sub\n");
            let mut parameter = "__ferro_repl_args".to_string();
            while source.text.to_ascii_lowercase().contains(&parameter)
                || self
                    .module
                    .globals
                    .iter()
                    .any(|v| v.name.eq_ignore_ascii_case(&parameter))
            {
                parameter.push('_');
            }
            source.push(&format!("Sub AppStart({parameter}() As String)\nEnd Sub\n"));
            for definition in &subs {
                source.cell(&definition.cell);
            }
            let parse = ferro_syntax::parse(&source.text);
            if !parse.errors().is_empty() {
                let errors = parse
                    .errors()
                    .iter()
                    .map(|e| source.diagnostic(e.range, &e.message))
                    .collect::<String>();
                return Err(errors);
            }
            let (module, diagnostics, bindings) =
                ferro_hir::check_session(&parse.syntax(), &self.bindings);
            if !diagnostics.is_empty() {
                return Err(diagnostics
                    .iter()
                    .map(|e| source.diagnostic(e.range, &e.message))
                    .collect::<String>());
            }
            Ok((source, module, bindings))
        };
        let result = if kind == FragmentKind::Expression {
            build(true).or_else(|echo_errors| build(false).map_err(|_| echo_errors))
        } else {
            build(false)
        };
        let (source, module, bindings) = match result {
            Ok(ok) => ok,
            Err(errors) => {
                let _ = err.write_all(errors.as_bytes());
                return Submission {
                    code: 3,
                    exited: false,
                };
            }
        };
        for global in module.globals.iter().skip(self.module.globals.len()) {
            if self
                .module
                .subs
                .iter()
                .filter(|sub| {
                    self.subs
                        .iter()
                        .any(|definition| definition.name.eq_ignore_ascii_case(&sub.name))
                })
                .any(|sub| {
                    sub.locals
                        .iter()
                        .any(|v| v.name.eq_ignore_ascii_case(&global.name))
                })
            {
                return reject(
                    "a new variable would change an existing Sub's local binding; use another name",
                    err,
                );
            }
        }
        for definition in &mut subs {
            let sub = module
                .subs
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(&definition.name))
                .unwrap();
            let signature = (
                sub.locals[..sub.params as usize]
                    .iter()
                    .map(|v| v.ty.clone())
                    .collect(),
                sub.ret.clone(),
            );
            if let Some(old) = self.subs.iter().find(|d| d.name == definition.name)
                && old.signature.as_ref() != Some(&signature)
            {
                return reject(
                    "a replacement Sub must keep its parameter and return types; use :reset",
                    err,
                );
            }
            definition.signature = Some(signature);
        }
        let program = ferro_mir::lower(&module);
        if let Err(message) = ferro_mir::validate(&program) {
            return reject(&message, err);
        }
        self.types = types;
        self.subs = subs;
        self.module = module;
        self.bindings = bindings;
        let result = self.state.execute(&program, out, err);
        if let Some(error) = &result.outcome.error {
            let message = if error.message.is_empty() {
                error.kind.clone()
            } else {
                format!("{}: {}", error.kind, error.message)
            };
            let _ = err.write_all(source.diagnostic(error.range, &message).as_bytes());
        }
        Submission {
            code: (result.outcome.exit_code & 255) as u8,
            exited: result.exited,
        }
    }
}

/// Runs an input stream. Interactive errors recover; scripts stop at first error.
/// Output errors are deliberately ignored, like the other ferro commands.
pub fn run(
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    interactive: bool,
) -> u8 {
    let mut session = Session::default();
    let mut pending = String::new();
    loop {
        if interactive {
            let _ = err.write_all(if pending.is_empty() { b"> " } else { b"... " });
            let _ = err.flush();
        }
        let mut line = String::new();
        match input.read_line(&mut line) {
            Err(error) => {
                let _ = writeln!(err, "ferro repl: cannot read input: {error}");
                return 2;
            }
            Ok(0) => {
                if !pending.is_empty() {
                    let result = session.submit(&pending, out, err);
                    return result.code;
                }
                return 0;
            }
            _ => {}
        }
        let open_string = ferro_syntax::lexer::lex_fragment(&pending).2;
        if !open_string && line.trim().starts_with(':') {
            match line.trim() {
                ":help" => {
                    let _ = out.write_all(HELP.as_bytes());
                }
                ":quit" => return 0,
                ":cancel" => pending.clear(),
                ":reset" => {
                    pending.clear();
                    session = Session::default();
                }
                _ => {
                    let _ = writeln!(
                        err,
                        "ferro repl: unknown command {}; use :help",
                        line.trim()
                    );
                    if !interactive {
                        return 2;
                    }
                }
            }
            continue;
        }
        pending.push_str(&line);
        let (_, _, status) = ferro_syntax::parse_fragment(&pending);
        if status == Completeness::Incomplete {
            continue;
        }
        let result = session.submit(&pending, out, err);
        pending.clear();
        let _ = out.flush();
        if result.exited || (!interactive && result.code != 0) {
            return result.code;
        }
    }
}
