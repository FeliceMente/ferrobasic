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

/// Classify the decoded URI path, leaving the original URI as the document key.
/// Query/fragment text and the authority are not part of the source file name.
fn is_project_uri(uri: &str) -> bool {
    let resource = uri.split(['?', '#']).next().unwrap_or(uri);
    let Some((_, rest)) = resource.split_once(':') else {
        return false;
    };
    let path = if let Some(authority_path) = rest.strip_prefix("//") {
        authority_path.split_once('/').map_or("", |(_, path)| path)
    } else {
        rest
    };
    let mut decoded = Vec::with_capacity(path.len());
    let mut bytes = path.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hex = |byte: u8| (byte as char).to_digit(16).map(|n| n as u8);
            let pair = bytes.next().and_then(hex).zip(bytes.next().and_then(hex));
            let Some((high, low)) = pair else {
                return false;
            };
            decoded.push(high * 16 + low);
        } else {
            decoded.push(byte);
        }
    }
    decoded
        .rsplit(|byte| *byte == b'/')
        .next()
        .is_some_and(|name| name.len() >= 4 && name[name.len() - 4..].eq_ignore_ascii_case(b".b4j"))
}

/// One pass builds line starts and cumulative UTF-8/UTF-16 differences.
/// Lookups never rescan the source, even for many errors on one long line.
struct PositionIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
    /// (byte offset after a non-ASCII character, cumulative byte/unit excess).
    utf16_excess: Vec<(usize, usize)>,
}

impl<'a> PositionIndex<'a> {
    fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        let mut utf16_excess = Vec::new();
        let mut excess = 0;
        for (at, ch) in text.char_indices() {
            match ch {
                '\r' => starts.push(
                    at + if text.as_bytes().get(at + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    },
                ),
                '\n' if at == 0 || text.as_bytes()[at - 1] != b'\r' => starts.push(at + 1),
                _ => {}
            }
            let difference = ch.len_utf8() - ch.len_utf16();
            if difference != 0 {
                excess += difference;
                utf16_excess.push((at + ch.len_utf8(), excess));
            }
        }
        Self {
            text,
            starts,
            utf16_excess,
        }
    }

    fn excess_at(&self, offset: usize) -> usize {
        let end = self.utf16_excess.partition_point(|&(at, _)| at <= offset);
        end.checked_sub(1).map_or(0, |i| self.utf16_excess[i].1)
    }

    /// CRLF is one line break; a position inside it is clamped to the preceding
    /// line's end. Lone CR and LF work too, and EOF ranges remain empty.
    fn position(&self, offset: usize, utf8: bool) -> Value {
        let mut end = offset.min(self.text.len());
        while !self.text.is_char_boundary(end) {
            end -= 1;
        }
        let line = self.starts.partition_point(|&start| start <= end) - 1;
        let start = self.starts[line];
        let mut line_end = self
            .starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        if line_end > start && self.text.as_bytes()[line_end - 1] == b'\n' {
            line_end -= 1;
        }
        if line_end > start && self.text.as_bytes()[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        end = end.min(line_end);
        let mut character = end - start;
        if !utf8 {
            character -= self.excess_at(end) - self.excess_at(start);
        }
        json!({"line":line,"character":character})
    }
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
            let project = is_project_uri(uri);
            let (_, errors) =
                ferro_cli::check(&doc.text, ferro_cli::header_errors(&doc.text, project));
            let positions = PositionIndex::new(&doc.text);
            errors
                .into_iter()
                .map(|(range, message)| {
                    json!({
                        "range": {"start":positions.position(usize::from(range.start()), utf8),
                                   "end":positions.position(usize::from(range.end()), utf8)},
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
    fn project_uri_components() {
        for uri in [
            "file:///App.b4j",
            "git:/App%2eb4%6A?ref=HEAD#main",
            "untitled:App.B4J",
        ] {
            assert!(is_project_uri(uri), "{uri}");
        }
        for uri in [
            "file://host.b4j",
            "file:///App.bas?name=App.b4j",
            "file:///App.bas#App.b4j",
            "file:///App.b4j%3Fref",
            "file:///App%xy.b4j",
            "file:///App%.b4j",
        ] {
            assert!(!is_project_uri(uri), "{uri}");
        }
    }

    #[test]
    fn unicode_columns_restart_after_line_breaks() {
        let index = PositionIndex::new("\u{feff}é😀x\r\n界😀z\ry\n");
        for (offset, line, utf16_column, utf8_column) in [
            (0, 0, 0, 0),
            (3, 0, 1, 3),
            (5, 0, 2, 5),
            (9, 0, 4, 9),
            (10, 0, 5, 10),
            (11, 0, 5, 10),
            (12, 1, 0, 0),
            (15, 1, 1, 3),
            (19, 1, 3, 7),
            (20, 1, 4, 8),
            (21, 2, 0, 0),
            (22, 2, 1, 1),
            (23, 3, 0, 0),
        ] {
            assert_eq!(
                index.position(offset, false),
                json!({"line":line,"character":utf16_column})
            );
            assert_eq!(
                index.position(offset, true),
                json!({"line":line,"character":utf8_column})
            );
        }
    }

    #[test]
    fn positions() {
        let text = "\u{feff}é😀x\r\ny\rz\n";
        let index = PositionIndex::new(text);
        assert_eq!(index.position(9, false), json!({"line":0,"character":4}));
        assert_eq!(index.position(9, true), json!({"line":0,"character":9}));
        assert_eq!(index.position(12, false), json!({"line":1,"character":0}));
        assert_eq!(index.position(14, false), json!({"line":2,"character":0}));
        assert_eq!(
            index.position(text.len(), false),
            json!({"line":3,"character":0})
        );
    }
}
