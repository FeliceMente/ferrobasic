//! Real stdio clients: every stdout byte must belong to an LSP frame.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

const GOOD: &str = "Sub Process_Globals\nEnd Sub\nSub AppStart (Args() As String)\nEnd Sub\n";
const URI: &str = "file:///does-not-exist/unsaved.bas";

struct Client {
    child: Child,
    input: Option<ChildStdin>,
    messages: Receiver<Value>,
}
impl Client {
    fn new() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ferro"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let (tx, messages) = mpsc::channel();
        std::thread::spawn(move || {
            loop {
                let mut line = String::new();
                if output.read_line(&mut line).unwrap() == 0 {
                    break;
                }
                let n: usize = line
                    .strip_prefix("Content-Length: ")
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                line.clear();
                output.read_line(&mut line).unwrap();
                assert_eq!(line, "\r\n");
                let mut body = vec![0; n];
                output.read_exact(&mut body).unwrap();
                tx.send(serde_json::from_slice(&body).unwrap()).unwrap();
            }
        });
        Self {
            child,
            input,
            messages,
        }
    }
    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).unwrap();
        let input = self.input.as_mut().unwrap();
        write!(input, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        input.write_all(&body).unwrap();
        input.flush().unwrap();
    }
    fn receive(&self) -> Value {
        self.messages
            .recv_timeout(Duration::from_secs(15))
            .expect("server response")
    }
    fn initialize(&mut self, encodings: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "capabilities":{"general":{"positionEncodings":encodings}},"rootUri":"file:///does-not-exist"}}));
        let response = self.receive();
        self.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
        assert_eq!(
            response["result"]["capabilities"]["textDocumentSync"]["change"],
            1
        );
        response
    }
    fn open(&mut self, uri: &str, text: &str) -> Value {
        self.send(
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
            "textDocument":{"uri":uri,"languageId":"ferrobasic","version":1,"text":text}}}),
        );
        self.receive()
    }
    fn change(&mut self, uri: &str, version: i64, text: &str) {
        self.send(
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
            "textDocument":{"uri":uri,"version":version},"contentChanges":[{"text":text}]}}),
        );
    }
    fn finish(mut self) {
        self.send(json!({"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}));
        assert_eq!(
            self.receive(),
            json!({"jsonrpc":"2.0","id":99,"result":null})
        );
        self.send(json!({"jsonrpc":"2.0","method":"exit"}));
        self.input.take();
        assert!(self.child.wait().unwrap().success());
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(stderr.is_empty(), "{stderr}");
        assert!(self.messages.recv_timeout(Duration::from_secs(1)).is_err());
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn diagnostics(value: &Value) -> &[Value] {
    assert_eq!(value["method"], "textDocument/publishDiagnostics");
    value["params"]["diagnostics"].as_array().unwrap()
}

#[test]
fn unsaved_syntax_type_incomplete_fix_and_close() {
    let mut c = Client::new();
    assert_eq!(
        c.initialize(Value::Null)["result"]["capabilities"]["positionEncoding"],
        "utf-16"
    );
    // A checker error in a file that does not exist on disk.
    let bad = GOOD.replace(
        "Sub AppStart (Args() As String)\n",
        "Sub AppStart (Args() As String)\nDim x As Int = True\n",
    );
    let response = c.open(URI, &bad);
    assert_eq!(response["params"]["uri"], URI);
    assert!(!diagnostics(&response).is_empty(), "{response}");
    c.change(URI, 2, "Sub AppStart(\n");
    assert!(!diagnostics(&c.receive()).is_empty());
    c.change(URI, 3, &GOOD.replace("End Sub\n", "End If\n"));
    assert!(!diagnostics(&c.receive()).is_empty());
    c.change(URI, 4, GOOD);
    let fixed = c.receive();
    assert!(diagnostics(&fixed).is_empty(), "{fixed}");
    assert_eq!(fixed["params"]["version"], 4);
    c.send(json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":URI}}}));
    assert!(diagnostics(&c.receive()).is_empty());
    // Late notifications for a closed document must not resurrect it.
    c.change(URI, 5, "bad");
    c.finish();
}

#[test]
fn rapid_edits_versions_and_reopen() {
    let mut c = Client::new();
    c.initialize(json!(["utf-32"]));
    c.open(URI, GOOD);
    for version in 2..30 {
        c.change(URI, version, if version % 2 == 0 { "Sub" } else { GOOD });
    }
    c.change(URI, 29, "bad duplicate");
    c.change(URI, 3, "bad stale");
    for version in 2..30 {
        let response = c.receive();
        assert_eq!(response["params"]["version"], version);
        assert_eq!(diagnostics(&response).is_empty(), version % 2 != 0);
    }
    c.send(json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":URI}}}));
    assert!(diagnostics(&c.receive()).is_empty());
    assert!(diagnostics(&c.open(URI, GOOD)).is_empty());
    c.finish();
}

#[test]
fn unicode_ranges_use_negotiated_units() {
    for (encodings, encoding, column) in
        [(Value::Null, "utf-16", 22), (json!(["utf-8"]), "utf-8", 24)]
    {
        let mut c = Client::new();
        assert_eq!(
            c.initialize(encodings)["result"]["capabilities"]["positionEncoding"],
            encoding
        );
        let text = GOOD.replace(
            "Sub AppStart (Args() As String)\n",
            "Sub AppStart (Args() As String)\nLog(\"😀\") : Log(Missing)\n",
        );
        let response = c.open(URI, &text);
        let missing = diagnostics(&response)
            .iter()
            .find(|d| d["message"].as_str().unwrap().contains("Missing"))
            .unwrap();
        assert_eq!(
            missing["range"]["start"],
            json!({"line":3,"character":column - 6}),
            "{response}"
        );
        assert_eq!(
            missing["range"]["end"],
            json!({"line":3,"character":column + 1}),
            "{response}"
        );
        c.finish();
    }
}

#[test]
fn project_headers_and_modules_remain_independent() {
    let mut c = Client::new();
    c.initialize(Value::Null);
    let project = "file:///virtual/App.B4J";
    let text = format!(
        "AppType=JavaFX\nLibrary1=jfx\nModule1=|relative|..\\Shared\\Tools\n@EndOfDesignText@\n{GOOD}"
    );
    let result = c.open(project, &text);
    let ds = diagnostics(&result);
    assert_eq!(ds.len(), 3, "{result}");
    for (i, d) in ds.iter().enumerate() {
        assert_eq!(d["range"]["start"]["line"], i);
    }
    assert!(ds[2]["message"].as_str().unwrap().contains("Level 1"));
    let module = c.open(
        "file:///virtual/Tools.bas",
        &format!("Type=Class\n@EndOfDesignText@\n{GOOD}"),
    );
    assert_eq!(diagnostics(&module).len(), 1);
    c.change(
        project,
        2,
        &format!("AppType=StandardJava\nLibrary1=jcore\n@EndOfDesignText@\n{GOOD}"),
    );
    assert!(diagnostics(&c.receive()).is_empty());
    // A module cannot borrow globals from another open document.
    let other = c.open(
        "untitled:other.bas",
        &GOOD.replace(
            "Sub AppStart (Args() As String)\n",
            "Sub AppStart (Args() As String)\nLog(UnknownGlobal)\n",
        ),
    );
    assert!(!diagnostics(&other).is_empty());
    c.finish();
}

#[test]
fn lifecycle_errors_and_unknown_methods() {
    let mut c = Client::new();
    c.send(json!({"jsonrpc":"2.0","id":0,"method":"example"}));
    assert_eq!(c.receive()["error"]["code"], -32002);
    c.initialize(Value::Null);
    c.send(json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover"}));
    assert_eq!(c.receive()["error"]["code"], -32601);
    c.finish();
}

#[test]
fn malformed_transport_fails_on_stderr_only() {
    for bytes in [
        "Content-Length: nope\r\n\r\n",
        "Content-Length: 999999999\r\n\r\n",
        "Content-Length: 1\r\n\r\nx",
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ferro"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(bytes.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("ferro lsp:"));
    }
}

#[test]
fn project_checks_follow_decoded_uri_paths() {
    let mut c = Client::new();
    c.initialize(Value::Null);
    let text = format!("AppType=JavaFX\nLibrary1=jfx\nModule1=Tools\n@EndOfDesignText@\n{GOOD}");
    let baseline = c.open("file:///virtual/App.b4j", &text);
    assert_eq!(diagnostics(&baseline).len(), 3);
    for uri in [
        "file:///virtual/App.b4%6a",
        "file:///virtual/App%2EB4J?revision=1#main",
        "git:/virtual/App.b4j?ref=HEAD",
        "untitled:App.b4j#main",
    ] {
        let response = c.open(uri, &text);
        assert_eq!(response["params"]["uri"], uri);
        assert_eq!(diagnostics(&response), diagnostics(&baseline), "{uri}");
        c.change(
            uri,
            2,
            &format!("AppType=StandardJava\nLibrary1=jcore\n@EndOfDesignText@\n{GOOD}"),
        );
        assert!(diagnostics(&c.receive()).is_empty());
    }
    c.finish();
}

#[test]
fn error_heavy_buffers_publish_then_clear() {
    let mut c = Client::new();
    c.initialize(Value::Null);
    // Large error lists previously took quadratic time to map. Exercise both
    // many lines and one long line; checking only line starts misses the latter.
    let count = 30_000;
    for (uri, separator, last_line, last_column) in [
        ("file:///many-lines.bas", "\n", count - 1, 0),
        ("file:///long-line.bas", ":", 0, (count - 1) * 4),
    ] {
        let response = c.open(uri, &format!("bad{separator}").repeat(count));
        let ds = diagnostics(&response);
        assert_eq!(ds.len(), count);
        assert_eq!(
            ds.last().unwrap()["range"]["start"],
            json!({"line":last_line,"character":last_column})
        );
        c.change(uri, 2, GOOD);
        assert!(diagnostics(&c.receive()).is_empty());
    }
    c.finish();
}
