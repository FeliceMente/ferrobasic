//! Terminal input only: the session engine and stream reader do not depend on
//! terminal state. Reedline leaves raw mode before returning any input to us.
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{IsTerminal, Write};

use crossterm::event::{KeyCode, KeyModifiers};
use reedline::{
    Emacs, FileBackedHistory, History, HistoryItem, Prompt, PromptEditMode, PromptHistorySearch,
    Reedline, ReedlineEvent, Signal, ValidationResult, Validator, default_emacs_keybindings,
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

#[derive(Default)]
struct Recall(VecDeque<String>);
impl Recall {
    fn remember(&mut self, text: &str) {
        // Remove the transport newline only; preserve BASIC whitespace and all
        // internal newlines. Failed submissions remain useful for correction.
        let text = text.strip_suffix('\n').unwrap_or(text);
        if text.trim().is_empty() || self.0.back().is_some_and(|last| last == text) {
            return;
        }
        if self.0.len() == HISTORY_LIMIT {
            self.0.pop_front();
        }
        self.0.push_back(text.to_owned());
    }

    fn install(&self, history: &mut dyn History) -> std::io::Result<()> {
        // Reedline automatically saves editor buffers, including control lines.
        // Rebuild its small navigation cache from the engine's submissions so
        // :help/:cancel and partial input never become recall entries.
        history.clear().map_err(std::io::Error::other)?;
        for text in &self.0 {
            history
                .save(HistoryItem::from_command_line(text.clone()))
                .map_err(std::io::Error::other)?;
        }
        Ok(())
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
struct EditorOutput(Option<filedescriptor::FileDescriptor>);
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
impl Drop for EditorOutput {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

struct TerminalInput {
    editor: Option<Reedline>,
    recall: Recall,
}
impl TerminalInput {
    fn new() -> Self {
        let mut bindings = default_emacs_keybindings();
        // EOF must also diagnose an incomplete editable block. Reedline's
        // default Ctrl+D deletes a character whenever the buffer is nonempty.
        bindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('d'),
            ReedlineEvent::ExecuteHostCommand(EOF_COMMAND.into()),
        );
        #[cfg(windows)]
        bindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('z'),
            ReedlineEvent::ExecuteHostCommand(EOF_COMMAND.into()),
        );
        bindings.add_binding(KeyModifiers::NONE, KeyCode::Tab, ReedlineEvent::None);
        Self {
            editor: Some(
                Reedline::create()
                    .with_edit_mode(Box::new(Emacs::new(bindings)))
                    .with_ansi_colors(false)
                    .with_history(Box::new(
                        FileBackedHistory::new(HISTORY_LIMIT).expect("bounded history"),
                    )),
            ),
            recall: Recall::default(),
        }
    }
}
impl Input for TerminalInput {
    fn read(&mut self, pending: &str, _: &mut dyn Write) -> std::io::Result<InputEvent> {
        let mut editor = self.editor.take().expect("editor returned after each read");
        self.recall.install(editor.history_mut())?;
        editor = editor.with_validator(Box::new(BasicValidator(pending.to_owned())));
        let mut output = EditorOutput::new()?;
        let result = editor.read_line(&BasicPrompt(!pending.is_empty()));
        output.restore()?;
        let event = match result {
            Ok(Signal::HostCommand(text)) if text == EOF_COMMAND => {
                Ok(InputEvent::Eof(editor.current_buffer_contents().to_owned()))
            }
            Ok(Signal::Success(mut text)) => {
                text.push('\n');
                Ok(InputEvent::Text(text))
            }
            Ok(Signal::CtrlD) => Ok(InputEvent::Eof(String::new())),
            // This ends the process after cleanup, rather than recovering at a
            // prompt. During BASIC execution the OS's ordinary Ctrl+C applies.
            Ok(Signal::CtrlC) => Ok(InputEvent::Interrupted),
            Ok(_) => Err(std::io::Error::other("unexpected terminal editor signal")),
            Err(error) => Err(error),
        };
        self.editor = Some(editor);
        event
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
        let mut recall = Recall::default();
        recall.remember("  \n");
        recall.remember("Sub F\n\nLog(1)\nEnd Sub\n");
        recall.remember("Sub F\n\nLog(1)\nEnd Sub\n");
        assert_eq!(recall.0.len(), 1);
        assert_eq!(recall.0[0], "Sub F\n\nLog(1)\nEnd Sub");
        recall.remember("1\n");
        recall.remember("2\n");
        recall.remember("1\n"); // older duplicates remain
        recall.remember(" 1\n"); // whitespace is significant for filtering
        assert_eq!(recall.0.len(), 5);
        assert_eq!(recall.0.back().unwrap(), " 1");
        for n in 0..HISTORY_LIMIT {
            recall.remember(&format!("{n}\n"));
        }
        let mut history = FileBackedHistory::new(HISTORY_LIMIT).unwrap();
        history
            .save(HistoryItem::from_command_line(":reset"))
            .unwrap();
        recall.install(&mut history).unwrap();
        let entries = history
            .search(SearchQuery::everything(SearchDirection::Forward, None))
            .unwrap();
        assert_eq!(entries.len(), HISTORY_LIMIT);
        assert_eq!(entries[0].command_line, "0");
        assert_eq!(entries.last().unwrap().command_line, "99");
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
