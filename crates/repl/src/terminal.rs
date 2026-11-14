//! Terminal input only: the session engine and stream reader do not depend on
//! terminal state. Reedline leaves raw mode before returning any input to us.
use std::borrow::Cow;
use std::io::{IsTerminal, Write};
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyModifiers};
use reedline::{
    Emacs, FileBackedHistory, History, HistoryItem, HistoryItemId, HistorySessionId, Prompt,
    PromptEditMode, PromptHistorySearch, Reedline, ReedlineEvent, SearchQuery, Signal,
    ValidationResult, Validator, default_emacs_keybindings,
};

use super::{Input, InputEvent, control_line, run_input};

const HISTORY_LIMIT: usize = 100;
const EOF_COMMAND: &str = "ferro:terminal-eof";

/// Editing needs stdin and stderr terminals. BASIC stdout may be redirected.
/// Missing TERM is normal on Windows; Unix needs an advertised terminal.
fn editing_supported(stdin_tty: bool, stderr_tty: bool, term: Option<&str>) -> bool {
    stdin_tty
        && stderr_tty
        && match term {
            Some(term) => !["", "dumb", "unknown", "emacs"]
                .iter()
                .any(|s| term.eq_ignore_ascii_case(s)),
            None => cfg!(windows),
        }
}

/// Selects terminal editing or the original plain stream reader.
pub fn run_terminal() -> u8 {
    let stdin = std::io::stdin();
    let interactive = stdin.is_terminal();
    let stderr = std::io::stderr();
    let term = std::env::var("TERM").ok();
    if !editing_supported(interactive, stderr.is_terminal(), term.as_deref()) {
        return super::run(
            &mut stdin.lock(),
            &mut std::io::stdout().lock(),
            &mut stderr.lock(),
            interactive,
        );
    }
    // Do not hold stdin's lock while the editor reads terminal events.
    let mut input = TerminalInput::new();
    let code = run_input(
        &mut input,
        &mut std::io::stdout().lock(),
        &mut stderr.lock(),
        true,
    );
    drop(input); // restore terminal state before returning an exit status
    code
}

// The engine explicitly records completed BASIC submissions. Reedline's own
// save() calls contain control lines/partial input, so its view is read-only.
#[derive(Clone)]
struct Recall(Arc<Mutex<FileBackedHistory>>);
impl Default for Recall {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(
            FileBackedHistory::new(HISTORY_LIMIT).expect("bounded in-memory history"),
        )))
    }
}
impl Recall {
    fn remember(&self, text: &str) {
        let text = text.strip_suffix('\n').unwrap_or(text);
        if !text.trim().is_empty() {
            self.0
                .lock()
                .expect("history lock")
                .save(HistoryItem::from_command_line(text))
                .expect("in-memory history save");
        }
    }
}
impl History for Recall {
    fn save(&mut self, item: HistoryItem) -> reedline::Result<HistoryItem> {
        Ok(item)
    }
    fn load(&self, id: HistoryItemId) -> reedline::Result<HistoryItem> {
        self.0.lock().expect("history lock").load(id)
    }
    fn count(&self, query: SearchQuery) -> reedline::Result<i64> {
        self.0.lock().expect("history lock").count(query)
    }
    fn search(&self, query: SearchQuery) -> reedline::Result<Vec<HistoryItem>> {
        self.0.lock().expect("history lock").search(query)
    }
    fn update(
        &mut self,
        id: HistoryItemId,
        f: &dyn Fn(HistoryItem) -> HistoryItem,
    ) -> reedline::Result<()> {
        self.0.lock().expect("history lock").update(id, f)
    }
    fn clear(&mut self) -> reedline::Result<()> {
        self.0.lock().expect("history lock").clear()
    }
    fn delete(&mut self, id: HistoryItemId) -> reedline::Result<()> {
        self.0.lock().expect("history lock").delete(id)
    }
    fn sync(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn session(&self) -> Option<HistorySessionId> {
        None
    }
}

struct BasicValidator(String);
impl Validator for BasicValidator {
    fn validate(&self, text: &str) -> ValidationResult {
        let mut pending = self.0.clone();
        for line in text.split_inclusive('\n') {
            if control_line(&pending, line) {
                return ValidationResult::Complete;
            }
            pending.push_str(line);
        }
        pending.push('\n');
        if ferro_syntax::parse_fragment(&pending).2 == ferro_syntax::Completeness::Incomplete {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

struct BasicPrompt(bool);
impl Prompt for BasicPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        "".into()
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        "".into()
    }
    fn render_prompt_indicator(&self, _: PromptEditMode) -> Cow<'_, str> {
        if self.0 { "... ".into() } else { "> ".into() }
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        "... ".into()
    }
    fn render_prompt_history_search_indicator(&self, _: PromptHistorySearch) -> Cow<'_, str> {
        "> ".into()
    }
}

// Crossterm's cursor-position query uses stdout even though Reedline paints
// on stderr. The CLI is serial: while reading, no BASIC or other output runs.
// Use safe descriptor APIs and restore before the session engine is entered.
#[cfg(unix)]
struct EditorOutput(Option<filedescriptor::FileDescriptor>);
#[cfg(unix)]
impl EditorOutput {
    fn new() -> std::io::Result<Self> {
        std::io::stdout().flush()?;
        let saved = filedescriptor::FileDescriptor::redirect_stdio(
            &std::io::stderr(),
            filedescriptor::StdioDescriptor::Stdout,
        )
        .map_err(std::io::Error::other)?;
        Ok(Self(Some(saved)))
    }
    fn restore(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()?;
        if let Some(saved) = &self.0 {
            filedescriptor::FileDescriptor::redirect_stdio(
                saved,
                filedescriptor::StdioDescriptor::Stdout,
            )
            .map_err(std::io::Error::other)?;
            self.0 = None;
        }
        Ok(())
    }
}
#[cfg(unix)]
impl Drop for EditorOutput {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

// Windows uses WinAPI for cursor queries. Redirecting there is unnecessary
// and can close a console handle shared by stdout and stderr.
#[cfg(not(unix))]
struct EditorOutput;
#[cfg(not(unix))]
impl EditorOutput {
    fn new() -> std::io::Result<Self> {
        Ok(Self)
    }
    fn restore(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn explicit_eof() -> ReedlineEvent {
    ReedlineEvent::ExecuteHostCommand(EOF_COMMAND.into())
}

// Keep fallible terminal setup/read/restoration separate from editor ownership.
fn read_editor(editor: &mut Reedline, prompt: &BasicPrompt) -> std::io::Result<Signal> {
    let mut output = EditorOutput::new()?;
    let result = editor.read_line(prompt);
    output.restore()?;
    result
}

struct TerminalInput {
    editor: Option<Reedline>,
    recall: Recall,
}
impl TerminalInput {
    fn new() -> Self {
        let mut bindings = default_emacs_keybindings();
        // Keep Emacs Ctrl+D (delete-forward, or EOF on an empty buffer).
        // Explicit EOF is separate so recalling/editing can never submit code.
        bindings.add_binding(
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            KeyCode::Char('d'),
            explicit_eof(),
        );
        #[cfg(windows)]
        bindings.add_binding(KeyModifiers::CONTROL, KeyCode::Char('z'), explicit_eof());
        bindings.add_binding(KeyModifiers::NONE, KeyCode::Tab, ReedlineEvent::None);
        let recall = Recall::default();
        Self {
            editor: Some(
                Reedline::create()
                    .with_edit_mode(Box::new(Emacs::new(bindings)))
                    .with_ansi_colors(false)
                    .with_history(Box::new(recall.clone())),
            ),
            recall,
        }
    }

    fn with_editor<T>(
        &mut self,
        pending: &str,
        read: impl FnOnce(Reedline) -> (Reedline, std::io::Result<T>),
    ) -> std::io::Result<T> {
        let editor = self.editor.take().expect("editor returned after each read");
        let editor = editor.with_validator(Box::new(BasicValidator(pending.to_owned())));
        // The callback returns ownership even if setup/read/restoration failed.
        let (editor, result) = read(editor);
        self.editor = Some(editor);
        result
    }
}
impl Input for TerminalInput {
    fn read(&mut self, pending: &str, _: &mut dyn Write) -> std::io::Result<InputEvent> {
        self.with_editor(pending, |mut editor| {
            let prompt = BasicPrompt(!pending.is_empty());
            let result = match read_editor(&mut editor, &prompt) {
                Ok(Signal::HostCommand(text)) if text == EOF_COMMAND => {
                    let text = editor.current_buffer_contents().to_owned();
                    // Host commands suspend the painter. Finish through Reedline's
                    // ordinary Submit exit path so it paints the rest of the buffer
                    // and moves below it, even if the cursor was on an earlier line.
                    // This only finalizes the UI: no BASIC executes here and the
                    // read-only history wrapper discards its automatic save.
                    editor = editor.with_immediately_accept(true);
                    let finish = read_editor(&mut editor, &prompt);
                    editor = editor.with_immediately_accept(false);
                    finish.map(|_| InputEvent::Eof(text))
                }
                Ok(Signal::Success(mut text)) => {
                    text.push('\n');
                    Ok(InputEvent::Text(text))
                }
                Ok(Signal::CtrlD) => Ok(InputEvent::Eof(String::new())),
                Ok(Signal::CtrlC) => Ok(InputEvent::Interrupted),
                Ok(_) => Err(std::io::Error::other("unexpected terminal editor signal")),
                Err(error) => Err(error),
            };
            (editor, result)
        })
    }
    fn remember(&mut self, text: &str) {
        self.recall.remember(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reedline::{SearchDirection, SearchQuery};

    #[test]
    fn recall_filters_bounds_and_preserves_multiline() {
        let recall = Recall::default();
        recall.remember("  \n");
        recall.remember("Sub F\n\nLog(1)\nEnd Sub\n");
        recall.remember("Sub F\n\nLog(1)\nEnd Sub\n");
        let entries = recall
            .search(SearchQuery::everything(SearchDirection::Forward, None))
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command_line, "Sub F\n\nLog(1)\nEnd Sub");
        recall.remember("1\n");
        recall.remember("2\n");
        recall.remember("1\n");
        recall.remember(" 1\n");
        assert_eq!(recall.count_all().unwrap(), 5);
        for n in 0..HISTORY_LIMIT {
            recall.remember(&format!("{n}\n"));
        }
        // The editor can see explicit submissions but cannot save its buffers.
        let mut history = recall.clone();
        history
            .save(HistoryItem::from_command_line(":reset"))
            .unwrap();
        history
            .save(HistoryItem::from_command_line("If True Then\n:cancel"))
            .unwrap();
        let entries = history
            .search(SearchQuery::everything(SearchDirection::Forward, None))
            .unwrap();
        assert_eq!(entries.len(), HISTORY_LIMIT);
        assert_eq!(entries[0].command_line, "0");
        assert_eq!(entries.last().unwrap().command_line, "99");
    }

    #[test]
    fn read_errors_return_editor_for_a_later_read() {
        let mut input = TerminalInput::new();
        for _ in 0..2 {
            assert!(
                input
                    .with_editor("", |editor| {
                        (
                            editor,
                            Err::<(), _>(std::io::Error::other("injected input failure")),
                        )
                    })
                    .is_err()
            );
            assert!(input.editor.is_some());
        }
        input
            .with_editor("", |editor| {
                assert_eq!(editor.current_buffer_contents(), "");
                (editor, Ok(()))
            })
            .unwrap();
    }

    #[test]
    fn completeness_and_commands_respect_strings() {
        let validator = BasicValidator(String::new());
        for text in ["Sub F", "Sub F\n", "If True Then\n\n", "\"a\n:cancel"] {
            assert!(
                matches!(validator.validate(text), ValidationResult::Incomplete),
                "{text}"
            );
        }
        for text in [
            "1 + 2",
            "Sub F\nEnd Sub",
            "If True Then\n:cancel",
            "If True Then\n:help",
            "\"a\n:quit\nb\"",
        ] {
            assert!(
                matches!(validator.validate(text), ValidationResult::Complete),
                "{text}"
            );
        }
        assert!(matches!(
            BasicValidator("If True Then\n".into()).validate("Log(1)"),
            ValidationResult::Incomplete
        ));
    }

    #[test]
    fn terminal_selection() {
        assert!(editing_supported(true, true, Some("xterm-256color")));
        assert!(!editing_supported(false, true, Some("xterm")));
        assert!(!editing_supported(true, false, Some("xterm")));
        for term in ["", "DUMB", "unknown", "emacs"] {
            assert!(!editing_supported(true, true, Some(term)));
        }
        assert_eq!(editing_supported(true, true, None), cfg!(windows));
    }
}
