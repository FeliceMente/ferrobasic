//! Diagnostics-only LSP. Full snapshots are checked serially: no worker can
//! publish results after a newer snapshot. No source is read from or written to disk.
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

const MAX_MESSAGE: usize = 16 * 1024 * 1024;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    let mut total = 0;
    loop {
        let mut line = String::new();
        // Bound headers as well as bodies, including a header with no newline.
        let n = std::io::Read::take(&mut *input, 8193).read_line(&mut line)?;
        if n == 0 {
            return if total == 0 {
                Ok(None)
            } else {
                Err(invalid("incomplete header"))
            };
        }
        total += n;
        if total > 8192 {
            return Err(invalid("header too large"));
        }
        if line == "\r\n" {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("invalid header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(invalid("duplicate Content-Length"));
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| invalid("invalid Content-Length"))?,
            );
        }
    }
    let length = length
        .filter(|n| *n <= MAX_MESSAGE)
        .ok_or_else(|| invalid("missing or oversized Content-Length"))?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(io::Error::other)
}

fn send(output: &mut impl Write, value: Value) -> io::Result<()> {
    let body = serde_json::to_vec(&value)?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

fn error(output: &mut impl Write, id: &Value, code: i32, message: &str) -> io::Result<()> {
    send(
        output,
        json!({"jsonrpc":"2.0", "id":id,"error":{"code":code,"message":message}}),
    )
}

struct Document {
    text: String,
    version: i64,
}

/// Byte offsets in compiler ranges become LSP code units. CRLF is one line
/// break; lone CR is also accepted by the compiler. EOF ranges remain empty.
fn position(text: &str, offset: usize, utf8: bool) -> Value {
    let mut end = offset.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let bytes = text.as_bytes();
    let (mut line, mut start, mut at) = (0, 0, 0);
    while at < end {
        if bytes[at] == b'\r' {
            at += 1;
            if bytes.get(at) == Some(&b'\n') {
                at += 1;
            }
            if at > end {
                break;
            }
            line += 1;
            start = at;
        } else if bytes[at] == b'\n' {
            at += 1;
            line += 1;
            start = at;
        } else {
            at += 1;
        }
    }
    let prefix = text[start..end].trim_end_matches(['\r', '\n']);
    let character = if utf8 {
        prefix.len()
    } else {
        prefix.encode_utf16().count()
    };
    json!({"line":line,"character":character})
}

fn publish(
    output: &mut impl Write,
    uri: &str,
    doc: Option<&Document>,
    utf8: bool,
) -> io::Result<()> {
    let diagnostics: Vec<_> = doc
        .map(|doc| {
            // The compiler does not link modules at Level 0. Preserve its project
            // header errors rather than loading unsupported siblings from disk.
            let project = uri
                .rsplit('/')
                .next()
                .unwrap_or(uri)
                .to_ascii_lowercase()
                .ends_with(".b4j");
            let (_, errors) =
                ferro_cli::check(&doc.text, ferro_cli::header_errors(&doc.text, project));
            errors
                .into_iter()
                .map(|(range, message)| {
                    json!({
                        "range": {"start":position(&doc.text, usize::from(range.start()), utf8),
                                   "end":position(&doc.text, usize::from(range.end()), utf8)},
                        "severity":1,"source":"ferro","message":message
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut params = json!({"uri":uri,"diagnostics":diagnostics});
    if let Some(doc) = doc {
        params["version"] = json!(doc.version);
    }
    send(
        output,
        json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":params}),
    )
}

/// Serve until exit or EOF. Exit without shutdown is unsuccessful, as LSP requires.
pub fn run(input: &mut impl BufRead, output: &mut impl Write) -> io::Result<u8> {
    let mut documents = HashMap::<String, Document>::new();
    let (mut initialized, mut shutdown, mut utf8) = (false, false, false);
    while let Some(message) = read(input)? {
        let id = message.get("id");
        let method = message["method"].as_str().unwrap_or("");
        if method == "exit" {
            return Ok(if shutdown { 0 } else { 1 });
        }
        if shutdown {
            if let Some(id) = id {
                error(output, id, -32600, "server has shut down")?;
            }
            continue;
        }
        if method == "initialize" && !initialized && id.is_some() {
            utf8 = message
                .pointer("/params/capabilities/general/positionEncodings")
                .and_then(Value::as_array)
                .is_some_and(|encodings| encodings.iter().any(|v| v == "utf-8"));
            initialized = true;
            send(
                output,
                json!({"jsonrpc":"2.0","id":id,"result":{
                    "capabilities":{"positionEncoding":if utf8 {"utf-8"} else {"utf-16"},
                    "textDocumentSync":{"openClose":true,"change":1}},
                    "serverInfo":{"name":"FerroBASIC","version":env!("CARGO_PKG_VERSION")}
                }}),
            )?;
            continue;
        }
        if !initialized {
            if let Some(id) = id {
                error(output, id, -32002, "server not initialized")?;
            }
            continue;
        }
        if method == "shutdown" && id.is_some() {
            shutdown = true;
            send(output, json!({"jsonrpc":"2.0","id":id,"result":null}))?;
            continue;
        }
        if let Some(id) = id {
            error(output, id, -32601, "method not supported")?;
            continue;
        }
        let params = &message["params"];
        let Some(uri) = params["textDocument"]["uri"].as_str() else {
            continue;
        };
        match method {
            "textDocument/didOpen" => {
                let doc = &params["textDocument"];
                if let (Some(text), Some(version)) = (doc["text"].as_str(), doc["version"].as_i64())
                {
                    if documents.contains_key(uri) {
                        continue;
                    }
                    documents.insert(
                        uri.into(),
                        Document {
                            text: text.into(),
                            version,
                        },
                    );
                    publish(output, uri, documents.get(uri), utf8)?;
                }
            }
            "textDocument/didChange" => {
                let Some(doc) = documents.get_mut(uri) else {
                    continue;
                };
                let Some(version) = params["textDocument"]["version"].as_i64() else {
                    continue;
                };
                if version <= doc.version {
                    continue;
                }
                let Some(changes) = params["contentChanges"].as_array() else {
                    continue;
                };
                // Full synchronization only. Never partially apply a malformed
                // update or interpret an incremental edit as a full snapshot.
                if changes.is_empty()
                    || changes
                        .iter()
                        .any(|c| c.get("range").is_some() || c["text"].as_str().is_none())
                {
                    continue;
                }
                doc.text = changes.last().unwrap()["text"].as_str().unwrap().into();
                doc.version = version;
                publish(output, uri, Some(doc), utf8)?;
            }
            "textDocument/didClose" => {
                documents.remove(uri);
                publish(output, uri, None, utf8)?;
            }
            _ => {}
        }
    }
    Ok(if shutdown { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positions() {
        let text = "\u{feff}é😀x\r\ny\rz\n";
        assert_eq!(position(text, 9, false), json!({"line":0,"character":4}));
        assert_eq!(position(text, 9, true), json!({"line":0,"character":9}));
        assert_eq!(position(text, 12, false), json!({"line":1,"character":0}));
        assert_eq!(position(text, 14, false), json!({"line":2,"character":0}));
        assert_eq!(
            position(text, text.len(), false),
            json!({"line":3,"character":0})
        );
    }
}
