use eudkit_core::map_context::CANONICAL_UNIT_NAMES;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const URI: &str = "file:///tmp/eudkit-lsp-test.py";
const TYPESCRIPT_URI: &str = "file:///tmp/eudkit-lsp-test.ts";
const TSX_URI: &str = "file:///tmp/eudkit-lsp-test.tsx";
const JAVASCRIPT_URI: &str = "file:///tmp/eudkit-lsp-test.js";
const JSX_URI: &str = "file:///tmp/eudkit-lsp-test.jsx";

struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl LspClient {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("eudkit-lsp should start");

        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout is piped"));

        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    fn initialize(&mut self) -> Value {
        let response = self.request("initialize", json!({ "capabilities": {} }));
        self.notify("initialized", json!({}));
        response
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));

        loop {
            let message = self.read_message();
            if message.get("id").and_then(Value::as_i64) == Some(id) {
                return message;
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body)
            .expect("request should be written");
        self.stdin.flush().expect("request should be flushed");
    }

    fn read_message(&mut self) -> Value {
        let mut content_length = None;

        loop {
            let mut line = String::new();
            let bytes_read = self
                .stdout
                .read_line(&mut line)
                .expect("response header should be readable");
            assert!(bytes_read > 0, "eudkit-lsp closed stdout unexpectedly");

            let header = line.trim_end_matches(['\r', '\n']);
            if header.is_empty() {
                break;
            }
            if let Some(value) = header.strip_prefix("Content-Length:") {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("Content-Length should be a number"),
                );
            }
        }

        let length = content_length.expect("response should include Content-Length");
        let mut body = vec![0; length];
        self.stdout
            .read_exact(&mut body)
            .expect("response body should be readable");
        serde_json::from_slice(&body).expect("response body should be JSON")
    }

    fn open(&mut self, text: &str) {
        self.open_document(URI, "python", text);
    }

    fn open_document(&mut self, uri: &str, language_id: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        );
    }

    fn change(&mut self, text: &str, version: i32) {
        self.change_document(URI, text, version);
    }

    fn change_document(&mut self, uri: &str, text: &str, version: i32) {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }],
            }),
        );
    }

    fn completion(&mut self, text: &str, byte_offset: usize) -> Vec<String> {
        self.completion_for(URI, text, byte_offset)
    }

    fn completion_for(&mut self, uri: &str, text: &str, byte_offset: usize) -> Vec<String> {
        let (line, character) = position_at(text, byte_offset);
        let response = self.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        );
        completion_labels(&response)
    }

    fn shutdown(mut self) {
        let response = self.request("shutdown", Value::Null);
        assert!(response["result"].is_null(), "shutdown should return null");
        self.notify("exit", Value::Null);
        drop(self.stdin);

        let status = self.child.wait().expect("eudkit-lsp should exit");
        assert!(status.success(), "eudkit-lsp exited with {status}");
    }
}

fn position_at(text: &str, byte_offset: usize) -> (u32, u32) {
    let before = &text[..byte_offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    let character = text[line_start..byte_offset].encode_utf16().count() as u32;
    (line, character)
}

fn completion_labels(response: &Value) -> Vec<String> {
    let Some(result) = response.get("result") else {
        return Vec::new();
    };
    if result.is_null() {
        return Vec::new();
    }

    let items = result
        .as_array()
        .or_else(|| result.get("items").and_then(Value::as_array))
        .expect("completion result should be an array or completion list");

    items
        .iter()
        .map(|item| {
            item.get("label")
                .and_then(Value::as_str)
                .expect("completion item should have a label")
                .to_string()
        })
        .collect()
}

fn assert_canonical_units(labels: &[String]) {
    let expected: HashSet<&str> = CANONICAL_UNIT_NAMES.iter().copied().collect();
    let actual: HashSet<&str> = labels.iter().map(String::as_str).collect();

    assert_eq!(labels.len(), 228, "expected exactly 228 canonical units");
    assert_eq!(actual.len(), labels.len(), "unit names should be unique");
    assert_eq!(actual, expected);

    for stock_filter in ["Any unit", "Men", "Buildings", "Factories"] {
        assert!(
            !actual.contains(stock_filter),
            "stock unit filter {stock_filter:?} must not be suggested"
        );
    }
    for custom_unit in ["UNI0", "UNI1", "UNIx"] {
        assert!(
            !actual.contains(custom_unit),
            "custom unit {custom_unit:?} must not be suggested"
        );
    }
    assert!(
        labels.iter().all(|label| !label.starts_with("UNI")),
        "custom UNIx names must not be suggested"
    );
}

#[test]
fn initializes_and_completes_canonical_units_for_unterminated_kill_unit_string() {
    let mut client = LspClient::spawn();
    let initialize = client.initialize();

    let capabilities = &initialize["result"]["capabilities"];
    assert_eq!(
        capabilities["completionProvider"]["triggerCharacters"],
        json!(["\"", "'", "("])
    );
    assert_eq!(capabilities["textDocumentSync"], json!(1));

    client.open("KillUnit(\"");
    let labels = client.completion("KillUnit(\"", "KillUnit(\"".len());
    assert_canonical_units(&labels);

    client.shutdown();
}

#[test]
fn completes_single_quotes_partial_values_and_closed_strings_after_did_change() {
    let mut client = LspClient::spawn();
    client.initialize();
    client.open("DisplayText(\"");

    let single_quote = "KillUnit('Mar";
    client.change(single_quote, 2);
    assert_canonical_units(&client.completion(single_quote, single_quote.len()));

    let partial = "KillUnit(\"Mar";
    client.change(partial, 3);
    assert_canonical_units(&client.completion(partial, partial.len()));

    let closed = "KillUnit(\"Terran Marine\", 1)";
    client.change(closed, 4);
    assert_canonical_units(&client.completion(closed, "KillUnit(\"Terr".len()));

    client.shutdown();
}

#[test]
fn does_not_complete_outside_the_kill_unit_unit_slot() {
    let mut client = LspClient::spawn();
    client.initialize();
    client.open("DisplayText(\"");

    let other_call = "DisplayText(\"";
    client.change(other_call, 2);
    assert!(client.completion(other_call, other_call.len()).is_empty());

    let player_argument = "KillUnit(\"Terran Marine\", \"";
    client.change(player_argument, 3);
    assert!(client
        .completion(player_argument, player_argument.len())
        .is_empty());

    let other_positional_argument = "KillUnit(1, \"";
    client.change(other_positional_argument, 4);
    assert!(client
        .completion(other_positional_argument, other_positional_argument.len())
        .is_empty());

    let comment = "# KillUnit(\"\n";
    client.change(comment, 5);
    assert!(client.completion(comment, "# KillUnit(\"".len()).is_empty());

    let free_form_string = "message = \"KillUnit(\"";
    client.change(free_form_string, 6);
    assert!(client
        .completion(free_form_string, free_form_string.len())
        .is_empty());

    let ordinary_code = "unit = \"Mar";
    client.change(ordinary_code, 7);
    assert!(client
        .completion(ordinary_code, ordinary_code.len())
        .is_empty());

    client.shutdown();
}

#[test]
fn completes_canonical_units_in_typescript_tsx_javascript_and_jsx() {
    let mut client = LspClient::spawn();
    client.initialize();

    let documents = [
        (TYPESCRIPT_URI, "typescript", "KillUnit(\""),
        (
            TSX_URI,
            "typescriptreact",
            "const view = <div />;\nKillUnit(\"",
        ),
        (JAVASCRIPT_URI, "javascript", "KillUnit(\""),
        (
            JSX_URI,
            "javascriptreact",
            "const view = <div />;\nKillUnit(\"",
        ),
    ];

    for (uri, language_id, text) in documents {
        client.open_document(uri, language_id, text);
        assert_canonical_units(&client.completion_for(uri, text, text.len()));
    }

    client.shutdown();
}

#[test]
fn completes_single_quotes_partial_values_and_closed_strings_in_typescript_and_javascript() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        client.open_document(uri, language_id, "DisplayText(\"");

        let single_quote = "KillUnit('Mar";
        client.change_document(uri, single_quote, 2);
        assert_canonical_units(&client.completion_for(uri, single_quote, single_quote.len()));

        let partial = "KillUnit(\"Mar";
        client.change_document(uri, partial, 3);
        assert_canonical_units(&client.completion_for(uri, partial, partial.len()));

        let closed = "KillUnit(\"Terran Marine\", 1)";
        client.change_document(uri, closed, 4);
        assert_canonical_units(&client.completion_for(uri, closed, "KillUnit(\"Terr".len()));
    }

    client.shutdown();
}

#[test]
fn does_not_complete_outside_the_unit_slot_in_typescript_and_javascript() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        client.open_document(uri, language_id, "DisplayText(\"");

        let cases = [
            ("DisplayText(\"", "DisplayText(\"".len()),
            (
                "KillUnit(\"Terran Marine\", \"",
                "KillUnit(\"Terran Marine\", \"".len(),
            ),
            ("KillUnit(1, \"", "KillUnit(1, \"".len()),
            ("// KillUnit(\"\nconst x = 1;", "// KillUnit(\"".len()),
            (
                "const message = \"KillUnit(\";",
                "const message = \"KillUnit(\"".len(),
            ),
            ("const unit = \"Mar", "const unit = \"Mar".len()),
            ("eud.KillUnit(\"Mar\", 1)", "eud.KillUnit(\"Mar".len()),
        ];

        for (version, (text, offset)) in cases.into_iter().enumerate() {
            client.change_document(uri, text, version as i32 + 2);
            assert!(
                client.completion_for(uri, text, offset).is_empty(),
                "must not complete in {text:?} for {language_id}"
            );
        }
    }

    client.shutdown();
}
