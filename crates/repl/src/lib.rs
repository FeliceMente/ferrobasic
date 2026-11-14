//! Stateful Level 0 submissions. Compiler transactions do not copy runtime
//! objects; successful compilation commits before execution starts.
use std::io::{BufRead, Write};

use ferro_hir::{Module, SessionBindings, Ty};
use ferro_syntax::{Completeness, FragmentKind, SyntaxKind, TextRange};

mod terminal;
pub use terminal::run_terminal;

pub const HELP: &str = "Enter BASIC statements, expressions, Types or complete Subs.\n:help    Show this help\n:cancel  Discard pending input\n:reset   Clear the session (keep input history)\n:quit    Leave the REPL\n";

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
    input_start: usize,
    input_end: usize,
    prefix_len: usize,
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
            input_start: 0,
            input_end: cell.text.len(),
            prefix_len: 0,
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
            .find(|s| s.start <= offset && offset < s.end)
            .or_else(|| self.spans.last());
        let Some(span) = span else {
            return format!("repl: error: {message}\n");
        };
        // Generated expression wrappers belong to the entered expression too:
        // runtime failures can point at Log rather than its argument.
        let offset = (offset.saturating_sub(span.start + span.prefix_len) + span.input_start)
            .min(span.input_end);
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
        if kind != FragmentKind::Type
            && root
                .descendants()
                .any(|n| n.kind() == SyntaxKind::TYPE_DECL)
        {
            return reject(
                "submit each Type definition separately, outside blocks and Subs",
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
                    let expression = root
                        .children()
                        .find(|node| node.kind() == SyntaxKind::EXPR_STMT)
                        .expect("expression fragment has an expression statement");
                    let range = expression.text_range();
                    let input_start = usize::from(range.start());
                    let input_end = usize::from(range.end());
                    let start = source.text.len();
                    source.push("Log(");
                    source.push(&cell.text[input_start..input_end]);
                    source.push(")\n");
                    source.spans.push(Span {
                        input_start,
                        input_end,
                        prefix_len: "Log(".len(),
                        start,
                        end: source.text.len(),
                        cell: cell.clone(),
                    });
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
    run_input(
        &mut StreamInput { input, interactive },
        out,
        err,
        interactive,
    )
}

enum InputEvent {
    Text(String),
    Eof(String),
    Interrupted,
}

trait Input {
    fn read(&mut self, pending: &str, err: &mut dyn Write) -> std::io::Result<InputEvent>;
    fn remember(&mut self, _text: &str) {}
}

struct StreamInput<'a> {
    input: &'a mut dyn BufRead,
    interactive: bool,
}

impl Input for StreamInput<'_> {
    fn read(&mut self, pending: &str, err: &mut dyn Write) -> std::io::Result<InputEvent> {
        if self.interactive {
            let _ = err.write_all(if pending.is_empty() { b"> " } else { b"... " });
            let _ = err.flush();
        }
        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            Ok(InputEvent::Eof(String::new()))
        } else {
            Ok(InputEvent::Text(line))
        }
    }
}

fn control_line(pending: &str, line: &str) -> bool {
    !ferro_syntax::lexer::lex_fragment(pending).2 && line.trim().starts_with(':')
}

/// Submits `pending` unless it is incomplete; returns the exit code when the
/// session ends.
fn submit_pending(
    session: &mut Session,
    pending: &mut String,
    input: &mut dyn Input,
    out: &mut dyn Write,
    err: &mut dyn Write,
    interactive: bool,
) -> Option<u8> {
    if ferro_syntax::parse_fragment(pending).2 == Completeness::Incomplete {
        return None;
    }
    input.remember(pending);
    let result = session.submit(pending, out, err);
    pending.clear();
    let _ = out.flush();
    (result.exited || (!interactive && result.code != 0)).then_some(result.code)
}

fn run_input(
    input: &mut dyn Input,
    out: &mut dyn Write,
    err: &mut dyn Write,
    interactive: bool,
) -> u8 {
    let mut session = Session::default();
    let mut pending = String::new();
    loop {
        let (text, eof) = match input.read(&pending, err) {
            Err(error) => {
                let _ = writeln!(err, "ferro repl: cannot read input: {error}");
                return 2;
            }
            Ok(InputEvent::Interrupted) => return 130,
            Ok(InputEvent::Eof(text)) => (text, true),
            Ok(InputEvent::Text(text)) => (text, false),
        };
        // A terminal read can contain an entire edited multiline buffer. Its
        // BASIC lines between control lines are checked and run as one
        // submission, so no complete prefix runs before an error further on;
        // a redirected read is one line, which makes no difference there.
        let mut fed = false;
        for line in text.split_inclusive('\n') {
            if control_line(&pending, line) {
                if std::mem::take(&mut fed)
                    && let Some(code) =
                        submit_pending(&mut session, &mut pending, input, out, err, interactive)
                {
                    return code;
                }
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
                let _ = out.flush();
                continue;
            }
            pending.push_str(line);
            fed = true;
        }
        if fed
            && let Some(code) =
                submit_pending(&mut session, &mut pending, input, out, err, interactive)
        {
            return code;
        }
        if eof {
            return if pending.is_empty() {
                0
            } else {
                session.submit(&pending, out, err).code
            };
        }
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use std::collections::VecDeque;

    struct Inputs {
        events: VecDeque<InputEvent>,
        history: Vec<String>,
    }
    impl Input for Inputs {
        fn read(&mut self, _: &str, _: &mut dyn Write) -> std::io::Result<InputEvent> {
            Ok(self
                .events
                .pop_front()
                .unwrap_or(InputEvent::Eof(String::new())))
        }
        fn remember(&mut self, text: &str) {
            self.history.push(text.to_owned());
        }
    }

    #[test]
    fn terminal_chunks_keep_command_and_session_semantics() {
        let mut input = Inputs {
            events: [
                "Dim n As Int = 7\n",
                "If True Then\n:help\n",
                "n = 8\n:cancel\n",
                "n\n",
                ":reset\n",
                "Sub F As Int\n\nReturn 9\nEnd Sub\n",
                "F()\n",
                "missing\n",
                "\"a\n:quit\nb\"\n",
                ":quit\n",
            ]
            .into_iter()
            .map(|s| InputEvent::Text(s.into()))
            .collect(),
            history: Vec::new(),
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run_input(&mut input, &mut out, &mut err, true), 0);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!("{HELP}7\n9\na\n:quit\nb\n")
        );
        assert!(String::from_utf8(err).unwrap().contains("<repl:3>"));
        assert_eq!(
            input.history,
            [
                "Dim n As Int = 7\n",
                "n\n",
                "Sub F As Int\n\nReturn 9\nEnd Sub\n",
                "F()\n",
                "missing\n",
                "\"a\n:quit\nb\"\n"
            ]
        );
    }

    #[test]
    fn edited_eof_diagnoses_incomplete_input_and_interrupt_ends_session() {
        for (event, expected) in [
            (InputEvent::Eof("If True Then\n".into()), 3),
            (InputEvent::Eof(String::new()), 0),
            (InputEvent::Interrupted, 130),
        ] {
            let mut input = Inputs {
                events: [event].into(),
                history: Vec::new(),
            };
            assert_eq!(
                run_input(&mut input, &mut Vec::new(), &mut Vec::new(), true),
                expected
            );
            assert!(input.history.is_empty());
        }
    }

    #[test]
    fn an_edited_buffer_is_checked_and_run_as_one_submission() {
        let run = |events: &[&str]| {
            let mut input = Inputs {
                events: events
                    .iter()
                    .map(|s| InputEvent::Text((*s).into()))
                    .collect(),
                history: Vec::new(),
            };
            let (mut out, mut err) = (Vec::new(), Vec::new());
            run_input(&mut input, &mut out, &mut err, true);
            (
                String::from_utf8(out).unwrap(),
                String::from_utf8(err).unwrap(),
                input.history,
            )
        };
        // A recalled Sub whose header was deleted: its complete first line
        // must not run before the unmatched `End Sub` is reported.
        let (out, err, history) = run(&["Log(\"unexpected\")\nEnd Sub\n"]);
        assert!(!out.contains("unexpected"), "{out}");
        assert!(err.contains("error"), "{err}");
        assert_eq!(history, ["Log(\"unexpected\")\nEnd Sub\n"]);
        // A valid edited buffer of several statements runs once, in order.
        let (out, err, history) = run(&["Dim a As Int = 1\nLog(a)\nLog(a + 1)\n"]);
        assert_eq!(out, "1\n2\n", "{err}");
        assert_eq!(history, ["Dim a As Int = 1\nLog(a)\nLog(a + 1)\n"]);
        // Control lines still split a buffer: what precedes runs first.
        let (out, _, history) = run(&["Log(1)\n:quit\nLog(2)\n"]);
        assert_eq!(out, "1\n");
        assert_eq!(history, ["Log(1)\n"]);
    }
}
