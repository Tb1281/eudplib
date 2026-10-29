use eudkit_core::canonical::CANONICAL_UNIT_NAMES;
use eudkit_core::{Chk, CompilationConfig, CompilationSession};
use eudplib_stormlib::{Archive, OpenArchiveFlags};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::Duration;

const URI: &str = "file:///tmp/eudkit-lsp-test.py";
const LOCATION_CALL: &str = "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"";
const SWITCH_CALL: &str = "SetSwitch(\"";
const TYPESCRIPT_URI: &str = "file:///tmp/eudkit-lsp-test.ts";
const TSX_URI: &str = "file:///tmp/eudkit-lsp-test.tsx";
const JAVASCRIPT_URI: &str = "file:///tmp/eudkit-lsp-test.js";
const JSX_URI: &str = "file:///tmp/eudkit-lsp-test.jsx";

struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
    next_id: i64,
    uri: String,
}

impl LspClient {
    fn spawn() -> Self {
        Self::spawn_with_uri(URI)
    }

    fn spawn_for(path: &Path) -> Self {
        Self::spawn_with_uri(&path_to_uri(path))
    }

    fn spawn_with_uri(uri: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("eudkit-lsp should start");

        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout is piped"));
        let stderr = child.stderr.take().expect("stderr is piped");

        Self {
            child,
            stdin,
            stdout,
            stderr: Some(stderr),
            next_id: 1,
            uri: uri.to_string(),
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
        let uri = self.uri.clone();
        self.open_at(&uri, text);
    }

    fn open_at(&mut self, uri: &str, text: &str) {
        self.open_document(uri, "python", text);
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
        let uri = self.uri.clone();
        self.change_at(&uri, text, version);
    }

    fn change_at(&mut self, uri: &str, text: &str, version: i32) {
        self.change_document(uri, text, version);
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
        let uri = self.uri.clone();
        self.completion_at(&uri, text, byte_offset)
    }

    fn completion_at(&mut self, uri: &str, text: &str, byte_offset: usize) -> Vec<String> {
        self.completion_for(uri, text, byte_offset)
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

    fn hover(&mut self, text: &str, byte_offset: usize) -> Option<Value> {
        let uri = self.uri.clone();
        self.hover_for(&uri, text, byte_offset)
    }

    fn hover_for(&mut self, uri: &str, text: &str, byte_offset: usize) -> Option<Value> {
        let (line, character) = position_at(text, byte_offset);
        let response = self.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        );
        assert!(
            response.get("error").is_none(),
            "hover returned an error: {response}"
        );
        response
            .get("result")
            .filter(|result| !result.is_null())
            .cloned()
    }

    fn signature_help(&mut self, text: &str, byte_offset: usize) -> Option<Value> {
        let uri = self.uri.clone();
        self.signature_help_for(&uri, text, byte_offset)
    }

    fn signature_help_for(&mut self, uri: &str, text: &str, byte_offset: usize) -> Option<Value> {
        let (line, character) = position_at(text, byte_offset);
        let response = self.request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        );
        assert!(
            response.get("error").is_none(),
            "signature help returned an error: {response}"
        );
        response
            .get("result")
            .filter(|result| !result.is_null())
            .cloned()
    }

    fn diagnostics(&mut self) -> Vec<Value> {
        let uri = self.uri.clone();
        self.diagnostics_for(&uri)
    }

    fn diagnostics_for(&mut self, uri: &str) -> Vec<Value> {
        loop {
            let message = self.read_message();
            if message.get("method").and_then(Value::as_str)
                != Some("textDocument/publishDiagnostics")
            {
                continue;
            }

            let params = &message["params"];
            if params.get("uri").and_then(Value::as_str) != Some(uri) {
                continue;
            }

            return params
                .get("diagnostics")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
    }

    fn close_document(&mut self, uri: &str) {
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        );
    }

    fn shutdown(mut self) -> String {
        let response = self.request("shutdown", Value::Null);
        assert!(response["result"].is_null(), "shutdown should return null");
        self.notify("exit", Value::Null);
        drop(self.stdin);

        let status = self.child.wait().expect("eudkit-lsp should exit");
        let mut stderr = String::new();
        if let Some(mut stderr_pipe) = self.stderr.take() {
            stderr_pipe
                .read_to_string(&mut stderr)
                .expect("stderr should be readable");
        }
        assert!(
            status.success(),
            "eudkit-lsp exited with {status}; stderr:\n{stderr}"
        );
        stderr
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

fn hover_markdown(hover: &Value) -> &str {
    hover["contents"]["value"]
        .as_str()
        .expect("hover should contain markdown contents")
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture(relative: &str) -> PathBuf {
    let path = repo_root().join(relative);
    assert!(path.exists(), "missing fixture {}", path.display());
    path
}

fn copy_fixture_to(relative: &str, target: &Path) {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::copy(fixture(relative), target).unwrap();
}

fn path_to_uri(path: &Path) -> String {
    url::Url::from_file_path(path)
        .expect("file path should convert to a file URI")
        .to_string()
}

fn write_project(directory: &Path, input_map: &str) -> PathBuf {
    let toml_path = directory.join("eudkit.toml");
    fs::write(
        &toml_path,
        format!(
            "[project]\nname = \"lsp-test\"\ninput_map = {input_map:?}\noutput_map = \"out.scx\"\n"
        ),
    )
    .unwrap();
    toml_path
}

fn reusable_switch_name(map_path: &Path) -> (String, u32) {
    let session = CompilationSession::from_map(map_path, CompilationConfig::default())
        .expect("fixture map should load");
    let data = session.map_data();

    for bytes in data.strings() {
        if bytes.is_empty()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
        {
            continue;
        }
        let Some(string_id) = data.string_table().find_id(bytes) else {
            continue;
        };
        let Some(name) = data.strings().get(string_id as usize - 1) else {
            continue;
        };
        let name = String::from_utf8(name.clone()).expect("filtered string should be ASCII");
        if name == "Anywhere" || name.starts_with("Switch ") {
            continue;
        }
        return (name, string_id);
    }

    panic!("fixture should contain a reusable string for a switch name");
}

fn replace_chk(map_path: &Path, chk_bytes: &[u8]) {
    Archive::set_file_locale(0);
    let mut archive = Archive::open(map_path, OpenArchiveFlags::empty()).unwrap();
    let mut tmp_chk = tempfile::NamedTempFile::new().unwrap();
    tmp_chk.write_all(chk_bytes).unwrap();
    tmp_chk.flush().unwrap();
    archive
        .add_file(tmp_chk.path(), "staredit\\scenario.chk", true)
        .unwrap();
    archive.compact().unwrap();
}

fn name_first_switch(map_path: &Path, string_id: u32) {
    Archive::set_file_locale(0);
    let mut archive = Archive::open(map_path, OpenArchiveFlags::empty()).unwrap();
    let raw_chk = {
        let mut file = archive.open_file("staredit\\scenario.chk").unwrap();
        file.read_all().unwrap()
    };
    let mut chk = Chk::parse(&raw_chk).unwrap();
    let mut swnm = chk
        .section(b"SWNM")
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| vec![0; 256 * 4]);
    swnm.resize(256 * 4, 0);
    swnm[0..4].copy_from_slice(&string_id.to_le_bytes());
    chk.set_section(b"SWNM", swnm).unwrap();
    drop(archive);
    replace_chk(map_path, &chk.to_bytes());
}

fn assert_location_fallback(labels: &[String]) {
    assert_eq!(
        labels,
        ["Anywhere".to_string()],
        "without a usable Map Context only the canonical Anywhere location should be suggested"
    );
}

fn assert_switch_canonical(labels: &[String]) {
    assert_eq!(labels.len(), 256, "expected 256 canonical switches");
    assert_eq!(labels.first().map(String::as_str), Some("Switch 1"));
    assert_eq!(labels.last().map(String::as_str), Some("Switch 256"));
    assert_eq!(
        labels.iter().collect::<HashSet<_>>().len(),
        labels.len(),
        "switch names should be unique"
    );
}

fn assert_warning(diagnostic: &Value, expected: &str) {
    assert_eq!(
        diagnostic["severity"],
        json!(2),
        "expected a warning diagnostic: {diagnostic}"
    );
    assert_eq!(diagnostic["source"], json!("eudkit"));

    let message = diagnostic["message"]
        .as_str()
        .expect("diagnostic should have a message");
    assert!(
        message.contains(expected),
        "expected {expected:?} in diagnostic message {message:?}"
    );
}

fn assert_range(diagnostic: &Value, text: &str, start: usize, end: usize) {
    let (line, character) = position_at(text, start);
    let (end_line, end_character) = position_at(text, end);

    assert_eq!(
        diagnostic["range"],
        json!({
            "start": { "line": line, "character": character },
            "end": { "line": end_line, "character": end_character },
        }),
        "unexpected diagnostic range"
    );
}

fn custom_unit_name(map_path: &Path) -> String {
    let session = CompilationSession::from_map(map_path, CompilationConfig::default())
        .expect("fixture map should load");

    session
        .map_data()
        .unit_settings()
        .iter()
        .filter_map(|unit| unit.custom_name.as_deref())
        .find(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|character| character.is_ascii_graphic() || character == ' ')
                && !name.contains(['"', '\'', '\\'])
        })
        .map(str::to_string)
        .expect("fixture should contain an ASCII custom unit name")
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
fn completes_map_locations_from_the_nearest_eudkit_toml() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    copy_fixture_to("test_maps/v2/verify_v2.scx", &project.join("map.scx"));
    write_project(&project, "map.scx");
    let document = project.join("src").join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open(LOCATION_CALL);
    let labels = client.completion(LOCATION_CALL, LOCATION_CALL.len());

    assert_eq!(
        labels,
        [
            "Anywhere".to_string(),
            "=== [EUDKIT V2 TEST MAP] ===".to_string(),
        ]
    );

    let keyword_call = "MoveLocation(location=\"";
    client.change(keyword_call, 2);
    let keyword_labels = client.completion(keyword_call, keyword_call.len());
    assert_eq!(keyword_labels, labels);

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
fn completes_map_switches_from_eudkit_toml() {
    let temp = tempfile::tempdir().unwrap();
    let map_path = temp.path().join("map.scx");
    copy_fixture_to("test_maps/v2/verify_v2.scx", &map_path);
    let (switch_name, string_id) = reusable_switch_name(&map_path);
    name_first_switch(&map_path, string_id);
    write_project(temp.path(), "map.scx");
    let document = temp.path().join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open(SWITCH_CALL);
    let labels = client.completion(SWITCH_CALL, SWITCH_CALL.len());

    assert_eq!(
        labels.len(),
        257,
        "expected one named switch plus 256 canonical switches"
    );
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.as_str() == switch_name.as_str())
            .count(),
        1,
        "named switch should be suggested exactly once"
    );
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.as_str() == "Switch 1")
            .count(),
        1,
        "canonical switch should be deduplicated"
    );
    assert_switch_canonical(&labels[1..]);

    client.shutdown();
}

#[test]
fn completes_map_locations_and_switches_in_typescript_and_javascript() {
    let temp = tempfile::tempdir().unwrap();
    let map_path = temp.path().join("map.scx");
    copy_fixture_to("test_maps/v2/verify_v2.scx", &map_path);
    let (switch_name, string_id) = reusable_switch_name(&map_path);
    name_first_switch(&map_path, string_id);
    write_project(temp.path(), "map.scx");

    for (file_name, language_id) in [("main.ts", "typescript"), ("main.js", "javascript")] {
        let document = temp.path().join(file_name);
        let uri = path_to_uri(&document);
        let mut client = LspClient::spawn_for(&document);
        client.initialize();

        client.open_document(&uri, language_id, LOCATION_CALL);
        let locations = client.completion_for(&uri, LOCATION_CALL, LOCATION_CALL.len());
        assert_eq!(
            locations,
            [
                "Anywhere".to_string(),
                "=== [EUDKIT V2 TEST MAP] ===".to_string(),
            ],
            "{language_id} should complete Map Context locations"
        );

        let closed_location = "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Nowhere\")";
        client.change_document(&uri, closed_location, 2);
        let closed_labels = client.completion_for(
            &uri,
            closed_location,
            closed_location.find("Nowhere").unwrap(),
        );
        assert_eq!(
            closed_labels, locations,
            "{language_id} should complete a closed location string"
        );

        client.change_document(&uri, SWITCH_CALL, 3);
        let switches = client.completion_for(&uri, SWITCH_CALL, SWITCH_CALL.len());
        assert_eq!(
            switches.len(),
            257,
            "{language_id} should suggest one named switch plus 256 canonical switches"
        );
        assert_eq!(
            switches
                .iter()
                .filter(|label| label.as_str() == switch_name.as_str())
                .count(),
            1,
            "named switch should be suggested exactly once"
        );
        assert_eq!(
            switches
                .iter()
                .filter(|label| label.as_str() == "Switch 1")
                .count(),
            1,
            "canonical switch should be deduplicated"
        );
        assert_switch_canonical(&switches[1..]);

        client.change_document(&uri, "KillUnit(\"", 4);
        assert_canonical_units(&client.completion_for(&uri, "KillUnit(\"", "KillUnit(\"".len()));

        client.shutdown();
    }
}

#[test]
fn typescript_unit_slot_stays_canonical_with_a_map_context() {
    let temp = tempfile::tempdir().unwrap();
    copy_fixture_to(
        "tests/outputmap/basemap/사이비2.scx",
        &temp.path().join("map.scx"),
    );
    write_project(temp.path(), "map.scx");
    let document = temp.path().join("main.ts");
    let uri = path_to_uri(&document);

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open_document(&uri, "typescript", "KillUnit(\"");
    assert_canonical_units(&client.completion_for(&uri, "KillUnit(\"", "KillUnit(\"".len()));

    client.shutdown();
}

#[test]
fn falls_back_to_canonical_symbols_without_eudkit_toml_in_typescript_and_javascript() {
    let temp = tempfile::tempdir().unwrap();

    for (file_name, language_id) in [("main.ts", "typescript"), ("main.js", "javascript")] {
        let document = temp.path().join(file_name);
        let uri = path_to_uri(&document);
        let mut client = LspClient::spawn_for(&document);
        client.initialize();

        client.open_document(&uri, language_id, LOCATION_CALL);
        assert_location_fallback(&client.completion_for(&uri, LOCATION_CALL, LOCATION_CALL.len()));

        client.change_document(&uri, SWITCH_CALL, 2);
        assert_switch_canonical(&client.completion_for(&uri, SWITCH_CALL, SWITCH_CALL.len()));

        client.shutdown();
    }
}

#[test]
fn reloads_typescript_map_context_after_eudkit_toml_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    copy_fixture_to("test_maps/v2/verify_v2.scx", &root.join("a.scx"));
    copy_fixture_to(
        "tests/outputmap/basemap/basemap_wireframe.scx",
        &root.join("b.scx"),
    );
    write_project(root, "a.scx");
    let document = root.join("main.ts");
    let uri = path_to_uri(&document);

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open_document(&uri, "typescript", LOCATION_CALL);
    let initial = client.completion_for(&uri, LOCATION_CALL, LOCATION_CALL.len());
    assert!(initial.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()));

    thread::sleep(Duration::from_millis(20));
    write_project(root, "b.scx");

    let reloaded = client.completion_for(&uri, LOCATION_CALL, LOCATION_CALL.len());
    assert!(
        reloaded.contains(&"Nowhere".to_string()),
        "TypeScript completion should use the updated project: {reloaded:?}"
    );
    assert!(
        !reloaded.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()),
        "TypeScript completion should drop the old map context"
    );

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
fn unit_slot_stays_canonical_only_with_a_map_context() {
    let temp = tempfile::tempdir().unwrap();
    copy_fixture_to(
        "tests/outputmap/basemap/사이비2.scx",
        &temp.path().join("map.scx"),
    );
    write_project(temp.path(), "map.scx");
    let document = temp.path().join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open("KillUnit(\"");
    let labels = client.completion("KillUnit(\"", "KillUnit(\"".len());

    assert_canonical_units(&labels);

    client.shutdown();
}

#[test]
fn falls_back_to_canonical_symbols_without_eudkit_toml() {
    let temp = tempfile::tempdir().unwrap();
    let document = temp.path().join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open("KillUnit(\"");
    assert_canonical_units(&client.completion("KillUnit(\"", "KillUnit(\"".len()));

    client.change(LOCATION_CALL, 2);
    assert_location_fallback(&client.completion(LOCATION_CALL, LOCATION_CALL.len()));

    client.change(SWITCH_CALL, 3);
    assert_switch_canonical(&client.completion(SWITCH_CALL, SWITCH_CALL.len()));

    client.shutdown();
}

#[test]
fn nearest_project_wins_for_each_document() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let nested = root.join("nested");

    copy_fixture_to(
        "tests/outputmap/basemap/basemap_wireframe.scx",
        &root.join("map.scx"),
    );
    copy_fixture_to("test_maps/v2/verify_v2.scx", &nested.join("map.scx"));
    write_project(root, "map.scx");
    write_project(&nested, "map.scx");

    let root_document = root.join("main.py");
    let nested_document = nested.join("main.py");
    let root_uri = path_to_uri(&root_document);
    let nested_uri = path_to_uri(&nested_document);

    let mut client = LspClient::spawn_for(&root_document);
    client.initialize();
    client.open_at(&root_uri, LOCATION_CALL);
    client.open_at(&nested_uri, LOCATION_CALL);

    let root_labels = client.completion_at(&root_uri, LOCATION_CALL, LOCATION_CALL.len());
    assert!(
        root_labels.contains(&"Nowhere".to_string()),
        "root document should use the root project: {root_labels:?}"
    );
    assert!(
        !root_labels.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()),
        "root document must not use the nested project"
    );

    let nested_labels = client.completion_at(&nested_uri, LOCATION_CALL, LOCATION_CALL.len());
    assert!(
        nested_labels.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()),
        "nested document should use the nearest project: {nested_labels:?}"
    );
    assert!(
        !nested_labels.contains(&"Nowhere".to_string()),
        "nested document must not use the root project"
    );

    client.shutdown();
}

#[test]
fn falls_back_to_canonical_locations_for_invalid_projects_and_maps() {
    let invalid_toml = tempfile::tempdir().unwrap();
    fs::write(invalid_toml.path().join("eudkit.toml"), "input_map =\n").unwrap();
    let mut client = LspClient::spawn_for(&invalid_toml.path().join("main.py"));
    client.initialize();
    client.open(LOCATION_CALL);
    assert_location_fallback(&client.completion(LOCATION_CALL, LOCATION_CALL.len()));
    let stderr = client.shutdown();
    assert!(
        stderr.contains("eudkit-lsp: failed to load"),
        "invalid eudkit.toml should be logged: {stderr}"
    );

    let missing_map = tempfile::tempdir().unwrap();
    write_project(missing_map.path(), "missing.scx");
    let mut client = LspClient::spawn_for(&missing_map.path().join("main.py"));
    client.initialize();
    client.open(LOCATION_CALL);
    assert_location_fallback(&client.completion(LOCATION_CALL, LOCATION_CALL.len()));
    assert_location_fallback(&client.completion(LOCATION_CALL, LOCATION_CALL.len()));
    let stderr = client.shutdown();
    assert_eq!(
        stderr
            .matches("eudkit-lsp: failed to load input map")
            .count(),
        1,
        "a failed map load should be logged once, not on every completion: {stderr}"
    );

    let malformed_chk = tempfile::tempdir().unwrap();
    let map_path = malformed_chk.path().join("map.scx");
    copy_fixture_to("test_maps/v2/verify_v2.scx", &map_path);
    replace_chk(&map_path, b"bad");
    write_project(malformed_chk.path(), "map.scx");
    let mut client = LspClient::spawn_for(&malformed_chk.path().join("main.py"));
    client.initialize();
    client.open(LOCATION_CALL);
    assert_location_fallback(&client.completion(LOCATION_CALL, LOCATION_CALL.len()));
    let stderr = client.shutdown();
    assert!(
        stderr.contains("eudkit-lsp: failed to load input map"),
        "malformed CHK should be logged: {stderr}"
    );
}

#[test]
fn reloads_map_context_after_eudkit_toml_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    copy_fixture_to("test_maps/v2/verify_v2.scx", &root.join("a.scx"));
    copy_fixture_to(
        "tests/outputmap/basemap/basemap_wireframe.scx",
        &root.join("b.scx"),
    );
    write_project(root, "a.scx");
    let document = root.join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open(LOCATION_CALL);
    let initial = client.completion(LOCATION_CALL, LOCATION_CALL.len());
    assert!(initial.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()));

    thread::sleep(Duration::from_millis(20));
    write_project(root, "b.scx");

    let reloaded = client.completion(LOCATION_CALL, LOCATION_CALL.len());
    assert!(
        reloaded.contains(&"Nowhere".to_string()),
        "completion should use the updated project: {reloaded:?}"
    );
    assert!(
        !reloaded.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()),
        "completion should drop the old map context"
    );

    client.shutdown();
}

#[test]
fn reloads_map_context_after_the_input_map_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let map_path = root.join("map.scx");
    copy_fixture_to("test_maps/v2/verify_v2.scx", &map_path);
    write_project(root, "map.scx");
    let document = root.join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();
    client.open(LOCATION_CALL);
    let initial = client.completion(LOCATION_CALL, LOCATION_CALL.len());
    assert!(initial.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()));

    thread::sleep(Duration::from_millis(20));
    copy_fixture_to("tests/outputmap/basemap/basemap_wireframe.scx", &map_path);

    let reloaded = client.completion(LOCATION_CALL, LOCATION_CALL.len());
    assert!(
        reloaded.contains(&"Nowhere".to_string()),
        "completion should use the updated input map: {reloaded:?}"
    );
    assert!(
        !reloaded.contains(&"=== [EUDKIT V2 TEST MAP] ===".to_string()),
        "completion should drop the old map context"
    );

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
            ("eudkit.DisplayText(\"Mar", "eudkit.DisplayText(\"Mar".len()),
            ("console.log(\"Mar", "console.log(\"Mar".len()),
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

#[test]
fn completes_qualified_supported_calls_in_typescript_and_javascript() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        let source = "eudkit.KillUnit(\"";
        client.open_document(uri, language_id, source);
        assert_canonical_units(&client.completion_for(uri, source, source.len()));

        let partial = "eudkit.KillUnit(\"Mar";
        client.change_document(uri, partial, 2);
        assert_canonical_units(&client.completion_for(uri, partial, partial.len()));
    }

    client.shutdown();
}

#[test]
fn initializes_with_hover_and_signature_help_capabilities() {
    let mut client = LspClient::spawn();
    let initialize = client.initialize();

    let capabilities = &initialize["result"]["capabilities"];
    assert_eq!(capabilities["hoverProvider"], json!(true));
    assert_eq!(
        capabilities["signatureHelpProvider"]["triggerCharacters"],
        json!(["(", ","])
    );
    assert_eq!(
        capabilities["signatureHelpProvider"]["retriggerCharacters"],
        json!([","])
    );

    client.shutdown();
}

#[test]
fn hovers_over_python_symbols_and_supported_calls() {
    let mut client = LspClient::spawn();
    client.initialize();

    let unit = "KillUnit(\"Terran Marine\", 1)";
    client.open(unit);
    let hover = client
        .hover(unit, unit.find("Terran Marine").unwrap() + 2)
        .expect("a canonical unit should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("**Terran Marine**"), "{text}");
    assert!(text.contains("Canonical unit · ID 0"), "{text}");

    let filter = "KillUnit(\"Any unit\", 1)";
    client.change(filter, 2);
    let hover = client
        .hover(filter, filter.find("Any unit").unwrap() + 2)
        .expect("a Stock Unit Filter should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("Stock Unit Filter · ID 229"), "{text}");

    let location = "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Anywhere\")";
    client.change(location, 3);
    let hover = client
        .hover(location, location.find("Anywhere").unwrap() + 2)
        .expect("a canonical location should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("Canonical location · ID 64"), "{text}");

    let switch = "SetSwitch(\"Switch 1\")";
    client.change(switch, 4);
    let hover = client
        .hover(switch, switch.find("Switch 1").unwrap() + 2)
        .expect("a canonical switch should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("Canonical switch · ID 0"), "{text}");

    let call = "KillUnit(\"Terran Marine\", 1)";
    client.change(call, 5);
    let hover = client
        .hover(call, call.find("KillUnit").unwrap() + 2)
        .expect("a supported call should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("**KillUnit(unit, player)**"), "{text}");
    assert!(text.contains("Returns `ActionSpec`."), "{text}");
    assert!(
        text.contains("`unit` accepts a canonical unit name"),
        "{text}"
    );

    client.shutdown();
}

#[test]
fn hovers_over_map_context_symbols_when_a_project_is_present() {
    let temp = tempfile::tempdir().unwrap();
    let map_path = temp.path().join("map.scx");
    copy_fixture_to("tests/outputmap/basemap/사이비2.scx", &map_path);
    let custom_unit = custom_unit_name(&map_path);
    write_project(temp.path(), "map.scx");
    let document = temp.path().join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();

    let source = format!("KillUnit(\"{custom_unit}\")\nMinimapPing(\"dvs1\")");
    client.open(&source);

    let hover = client
        .hover(&source, source.find(&custom_unit).unwrap() + 1)
        .expect("a map-context unit should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("Map Context unit"), "{text}");

    let hover = client
        .hover(&source, source.find("dvs1").unwrap() + 1)
        .expect("a map-context location should have hover");
    let text = hover_markdown(&hover);
    assert!(text.contains("Map Context location"), "{text}");
    assert!(text.contains("Bounds:"), "{text}");

    client.shutdown();
}

#[test]
fn hovers_over_typescript_and_javascript_units_and_qualified_calls() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        let source = "eudkit.KillUnit(\"Terran Marine\", 1);";
        client.open_document(uri, language_id, source);

        let hover = client
            .hover_for(uri, source, source.find("Terran Marine").unwrap() + 2)
            .unwrap_or_else(|| panic!("{language_id} unit should have hover"));
        let text = hover_markdown(&hover);
        assert!(
            text.contains("Canonical unit · ID 0"),
            "{language_id}: {text}"
        );

        let hover = client
            .hover_for(uri, source, source.find("KillUnit").unwrap() + 2)
            .unwrap_or_else(|| panic!("{language_id} call should have hover"));
        let text = hover_markdown(&hover);
        assert!(
            text.contains("**KillUnit(unit, player)**"),
            "{language_id}: {text}"
        );
        assert!(text.contains("Returns `Action`."), "{language_id}: {text}");
    }

    client.shutdown();
}

#[test]
fn completes_and_hovers_legacy_qualified_calls_in_typescript_and_javascript() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        let source = "eud.KillUnit(\"Mar";
        client.open_document(uri, language_id, source);
        assert_canonical_units(&client.completion_for(uri, source, source.len()));

        let closed = "eud.KillUnit(\"Terran Marine\", 1);";
        client.change_document(uri, closed, 2);
        let hover = client
            .hover_for(uri, closed, closed.find("KillUnit").unwrap() + 2)
            .unwrap_or_else(|| panic!("{language_id} legacy qualified call should have hover"));
        let text = hover_markdown(&hover);
        assert!(
            text.contains("**KillUnit(unit, player)**"),
            "{language_id}: {text}"
        );
        assert!(text.contains("Returns `Action`."), "{language_id}: {text}");
    }

    client.shutdown();
}

#[test]
fn provides_python_signature_help_at_calls_and_keyword_arguments() {
    let mut client = LspClient::spawn();
    client.initialize();

    let open_call = "KillUnit(";
    client.open(open_call);
    let signature = client
        .signature_help(open_call, open_call.len())
        .expect("KillUnit( should have signature help");
    assert_eq!(
        signature["signatures"][0]["label"],
        "KillUnit(unit, player)"
    );
    assert_eq!(signature["activeParameter"], 0);

    let after_comma = "KillUnit(\"Terran Marine\", ";
    client.change(after_comma, 2);
    let signature = client
        .signature_help(after_comma, after_comma.len())
        .expect("a call after a comma should have signature help");
    assert_eq!(
        signature["signatures"][0]["label"],
        "KillUnit(unit, player)"
    );
    assert_eq!(signature["activeParameter"], 1);

    let keyword = "KillUnit(unit=\"";
    client.change(keyword, 3);
    let signature = client
        .signature_help(keyword, keyword.len())
        .expect("a keyword argument should have signature help");
    assert_eq!(
        signature["signatures"][0]["label"],
        "KillUnit(unit, player)"
    );
    assert_eq!(signature["activeParameter"], 0);

    let keyword_after_positional = "KillUnit(player=1, unit=\"";
    client.change(keyword_after_positional, 4);
    let signature = client
        .signature_help(keyword_after_positional, keyword_after_positional.len())
        .expect("a keyword argument should override its positional index");
    assert_eq!(
        signature["signatures"][0]["label"],
        "KillUnit(unit, player)"
    );
    assert_eq!(signature["activeParameter"], 0);
    assert!(
        signature["signatures"][0]["documentation"]["value"]
            .as_str()
            .unwrap()
            .contains("Returns `ActionSpec`."),
        "{signature}"
    );

    let switch = "SetSwitch(";
    client.change(switch, 5);
    let signature = client
        .signature_help(switch, switch.len())
        .expect("SetSwitch( should have signature help");
    assert_eq!(
        signature["signatures"][0]["label"],
        "SetSwitch(switch, state)"
    );
    assert_eq!(signature["activeParameter"], 0);

    let keyword = "MoveLocation(dest_location=\"";
    client.change(keyword, 6);
    let signature = client
        .signature_help(keyword, keyword.len())
        .expect("a Python keyword argument should select its parameter");
    assert_eq!(
        signature["signatures"][0]["label"],
        "MoveLocation(location, unit, player, dest_location)"
    );
    assert_eq!(signature["activeParameter"], 3);

    client.shutdown();
}

#[test]
fn provides_signature_help_for_qualified_typescript_and_javascript_calls() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        let open_call = "eudkit.KillUnit(";
        client.open_document(uri, language_id, open_call);
        let signature = client
            .signature_help_for(uri, open_call, open_call.len())
            .unwrap_or_else(|| panic!("{language_id} qualified call should have signature help"));
        assert_eq!(
            signature["signatures"][0]["label"],
            "KillUnit(unit, player)"
        );
        assert_eq!(signature["activeParameter"], 0);

        let after_comma = "eudkit.KillUnit(\"Terran Marine\", ";
        client.change_document(uri, after_comma, 2);
        let signature = client
            .signature_help_for(uri, after_comma, after_comma.len())
            .unwrap_or_else(|| panic!("{language_id} comma should advance the parameter"));
        assert_eq!(
            signature["signatures"][0]["label"],
            "KillUnit(unit, player)"
        );
        assert_eq!(signature["activeParameter"], 1);
        assert!(
            signature["signatures"][0]["documentation"]["value"]
                .as_str()
                .unwrap()
                .contains("Returns `Action`."),
            "{language_id}: {signature}"
        );

        let switch = "eudkit.SetSwitch(";
        client.change_document(uri, switch, 3);
        let signature = client
            .signature_help_for(uri, switch, switch.len())
            .unwrap_or_else(|| {
                panic!("{language_id} qualified SetSwitch should have signature help")
            });
        assert_eq!(
            signature["signatures"][0]["label"], "SetSwitch(switchId, action)",
            "{language_id}: {signature}"
        );
        assert_eq!(
            signature["activeParameter"], 0,
            "{language_id}: {signature}"
        );

        let keyword = "eudkit.MoveLocation(destLocation=\"";
        client.change_document(uri, keyword, 4);
        let signature = client
            .signature_help_for(uri, keyword, keyword.len())
            .unwrap_or_else(|| {
                panic!("{language_id} qualified MoveLocation should have signature help")
            });
        assert_eq!(
            signature["signatures"][0]["label"],
            "MoveLocation(location, onUnit, owner, destLocation)",
            "{language_id}: {signature}"
        );
        assert_eq!(
            signature["activeParameter"], 3,
            "{language_id}: {signature}"
        );
    }

    client.shutdown();
}

#[test]
fn returns_no_hover_or_signature_help_outside_supported_contexts() {
    let mut client = LspClient::spawn();
    client.initialize();

    let unknown = "KillUnit(\"Not A Unit\", 1)";
    client.open(unknown);
    assert!(
        client
            .hover(unknown, unknown.find("Not A Unit").unwrap() + 2)
            .is_none(),
        "an unknown symbol should not have hover"
    );

    let non_call = "value = 1";
    client.change(non_call, 2);
    assert!(
        client.hover(non_call, 2).is_none(),
        "a non-call identifier should not have hover"
    );

    let unsupported_call = "DisplayText(";
    client.change(unsupported_call, 3);
    assert!(
        client
            .signature_help(unsupported_call, unsupported_call.len())
            .is_none(),
        "an unsupported call should not have signature help"
    );

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        for unsupported in [
            "console.log(\"Not A Unit\")",
            "eudkit.DisplayText(\"Not A Unit\")",
        ] {
            client.open_document(uri, language_id, unsupported);
            assert!(
                client
                    .hover_for(
                        uri,
                        unsupported,
                        unsupported.find("Not A Unit").unwrap() + 2
                    )
                    .is_none(),
                "{language_id} unsupported member call should not have hover: {unsupported}"
            );
            assert!(
                client
                    .signature_help_for(uri, unsupported, unsupported.find('(').unwrap() + 1)
                    .is_none(),
                "{language_id} unsupported member call should not have signature help: {unsupported}"
            );
        }
    }

    client.shutdown();
}

#[test]
fn warns_about_unknown_map_symbols_in_python_and_clears_them() {
    let mut client = LspClient::spawn();
    client.initialize();

    let unknown_unit = "KillUnit(\"Not A Unit\")";
    client.open(unknown_unit);
    let diagnostics = client.diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_warning(&diagnostics[0], "Unknown unit \"Not A Unit\"");
    let unit_start = unknown_unit.find('"').unwrap();
    assert_range(
        &diagnostics[0],
        unknown_unit,
        unit_start,
        unknown_unit.rfind('"').unwrap() + 1,
    );

    let valid = "KillUnit(\"Terran Marine\")";
    client.change(valid, 2);
    assert!(
        client.diagnostics().is_empty(),
        "fixing the unit should clear its warning"
    );

    let unknown_location = "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Not A Location\")";
    client.change(unknown_location, 3);
    let diagnostics = client.diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_warning(&diagnostics[0], "Unknown location \"Not A Location\"");
    let location_start = unknown_location.find("\"Not A Location\"").unwrap();
    assert_range(
        &diagnostics[0],
        unknown_location,
        location_start,
        location_start + "\"Not A Location\"".len(),
    );

    let unknown_switch = "SetSwitch(\"Not A Switch\")";
    client.change(unknown_switch, 4);
    let diagnostics = client.diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_warning(&diagnostics[0], "Unknown switch \"Not A Switch\"");

    client.close_document(URI);
    assert!(
        client.diagnostics_for(URI).is_empty(),
        "closing the document should clear its warnings"
    );

    client.shutdown();
}

#[test]
fn accepts_numeric_stock_filter_and_free_form_symbols_in_python() {
    let mut client = LspClient::spawn();
    client.initialize();

    let source = concat!(
        "KillUnit(229)\n",
        "KillUnit(\"Any unit\")\n",
        "KillUnit(\"Men\")\n",
        "KillUnit(\"Buildings\")\n",
        "KillUnit(\"Factories\")\n",
        "KillUnit(\"300\")\n",
        "KillUnit(\"-1\")\n",
        "DisplayText(\"Not A Unit\")\n",
        "MinimapPing(\"Anywhere\")\n",
        "MinimapPing(\"Location 1\")\n",
        "MinimapPing(\"Location 255\")\n",
        "SetSwitch(\"Switch 1\")\n",
        "SetSwitch(\"Switch 256\")\n",
    );

    client.open(source);
    assert!(
        client.diagnostics().is_empty(),
        "valid symbols and free-form strings should not warn"
    );

    client.shutdown();
}

#[test]
fn accepts_known_custom_units_and_named_locations_with_a_map_context() {
    let temp = tempfile::tempdir().unwrap();
    let map_path = temp.path().join("map.scx");
    copy_fixture_to("tests/outputmap/basemap/사이비2.scx", &map_path);
    let custom_unit = custom_unit_name(&map_path);
    write_project(temp.path(), "map.scx");
    let document = temp.path().join("main.py");

    let mut client = LspClient::spawn_for(&document);
    client.initialize();

    let valid =
        format!("KillUnit(\"{custom_unit}\")\nMinimapPing(\"dvs1\")\nSetSwitch(\"Switch 1\")");
    client.open(&valid);
    assert!(
        client.diagnostics().is_empty(),
        "known custom unit {custom_unit:?} and named location should not warn"
    );

    let unknown = format!("KillUnit(\"{custom_unit} Unknown\")");
    client.change(&unknown, 2);
    let diagnostics = client.diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_warning(&diagnostics[0], "Unknown unit");

    client.shutdown();
}

#[test]
fn warns_about_unknown_map_symbols_in_typescript_and_javascript_and_clears_them() {
    let mut client = LspClient::spawn();
    client.initialize();

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        let source = concat!(
            "KillUnit(\"Not A Unit\");\n",
            "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Not A Location\");\n",
            "SetSwitch(\"Not A Switch\");\n",
        );
        client.open_document(uri, language_id, source);
        let diagnostics = client.diagnostics_for(uri);
        assert_eq!(diagnostics.len(), 3, "{language_id}");
        assert_warning(&diagnostics[0], "Unknown unit");
        assert_warning(&diagnostics[1], "Unknown location");
        assert_warning(&diagnostics[2], "Unknown switch");

        client.change_document(uri, "KillUnit(\"Terran Marine\");", 2);
        assert!(
            client.diagnostics_for(uri).is_empty(),
            "{language_id} should clear fixed warnings"
        );

        client.change_document(uri, "KillUnit(\"Not A Unit\");", 3);
        assert_eq!(
            client.diagnostics_for(uri).len(),
            1,
            "{language_id} should warn again after the fix is reverted"
        );

        client.close_document(uri);
        assert!(
            client.diagnostics_for(uri).is_empty(),
            "{language_id} should clear warnings on close"
        );
    }

    client.shutdown();
}

#[test]
fn accepts_numeric_stock_filter_and_free_form_symbols_in_typescript_and_javascript() {
    let mut client = LspClient::spawn();
    client.initialize();

    let source = concat!(
        "KillUnit(229);\n",
        "KillUnit(\"Any unit\");\n",
        "KillUnit(\"Men\");\n",
        "KillUnit(\"Buildings\");\n",
        "KillUnit(\"Factories\");\n",
        "KillUnit(\"300\");\n",
        "KillUnit(\"-1\");\n",
        "DisplayText(\"Not A Unit\");\n",
        "MinimapPing(\"Anywhere\");\n",
        "MinimapPing(\"Location 1\");\n",
        "SetSwitch(\"Switch 256\");\n",
    );

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (JAVASCRIPT_URI, "javascript"),
    ] {
        client.open_document(uri, language_id, source);
        assert!(
            client.diagnostics_for(uri).is_empty(),
            "{language_id} should not warn about valid symbols"
        );

        client.close_document(uri);
        let _ = client.diagnostics_for(uri);
    }

    client.shutdown();
}

#[test]
fn accepts_known_custom_units_in_typescript_with_a_map_context() {
    let temp = tempfile::tempdir().unwrap();
    let map_path = temp.path().join("map.scx");
    copy_fixture_to("tests/outputmap/basemap/사이비2.scx", &map_path);
    let custom_unit = custom_unit_name(&map_path);
    write_project(temp.path(), "map.scx");

    for (file_name, language_id) in [("main.ts", "typescript"), ("main.js", "javascript")] {
        let document = temp.path().join(file_name);
        let uri = path_to_uri(&document);
        let mut client = LspClient::spawn_for(&document);
        client.initialize();

        let valid = format!("KillUnit(\"{custom_unit}\");");
        client.open_document(&uri, language_id, &valid);
        assert!(
            client.diagnostics_for(&uri).is_empty(),
            "{language_id} should accept known custom unit {custom_unit:?}"
        );

        let unknown = format!("KillUnit(\"{custom_unit} Unknown\");");
        client.change_document(&uri, &unknown, 2);
        let diagnostics = client.diagnostics_for(&uri);
        assert_eq!(diagnostics.len(), 1, "{language_id}");
        assert_warning(&diagnostics[0], "Unknown unit");

        client.shutdown();
    }
}

#[test]
fn warns_when_group_players_use_eud_offsets_in_python() {
    let mut client = LspClient::spawn();
    client.initialize();

    let source = concat!(
        "Deaths(Foes, Exactly, 15, 300)\n",
        "DeathsX(AllPlayers, Exactly, 15, 300)\n",
        "SetDeaths(c.Foes, Exactly, 15, 0x12C)\n",
        "SetDeathsX(eud.AllPlayers, Exactly, 15, +300)\n",
    );
    client.open(source);
    let diagnostics = client.diagnostics();
    assert_eq!(diagnostics.len(), 4);

    for (diagnostic, player) in
        diagnostics
            .iter()
            .zip(["Foes", "AllPlayers", "c.Foes", "eud.AllPlayers"])
    {
        assert_warning(diagnostic, "ambiguous");
        assert_warning(diagnostic, player.rsplit('.').next().unwrap());
        assert_warning(diagnostic, "Player1");

        let start = source.find(player).unwrap();
        assert_range(diagnostic, source, start, start + player.len());
    }

    let valid = concat!(
        "Deaths(Foes, Exactly, 15, \"Any unit\")\n",
        "Deaths(Foes, Exactly, 15, 229)\n",
        "Deaths(Foes, Exactly, 15, 0)\n",
        "Deaths(Foes, Exactly, 15, 232)\n",
        "Deaths(Player1, Exactly, 15, 300)\n",
        "Deaths(CurrentPlayer, Exactly, 15, 300)\n",
        "Deaths(player, Exactly, 15, 300)\n",
        "Deaths(Foe, Exactly, 15, 300)\n",
        "Deaths(Foes + 1, Exactly, 15, 300)\n",
        "Deaths(Foes, Exactly, 15, 232 + 1)\n",
        "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Anywhere\")\n",
    );
    client.change(valid, 2);
    assert!(
        client.diagnostics().is_empty(),
        "stock players, filters, variables, and arithmetic must not warn"
    );

    client.change("Deaths(Foes, Exactly, 15, 300)", 3);
    assert_eq!(
        client.diagnostics().len(),
        1,
        "the warning should return after the ambiguity is reintroduced"
    );

    client.shutdown();
}

#[test]
fn warns_when_group_players_use_eud_offsets_in_typescript_and_javascript() {
    let mut client = LspClient::spawn();
    client.initialize();

    let source = concat!(
        "Deaths(Foes, Exactly, 15, 300);\n",
        "DeathsX(AllPlayers, Exactly, 15, 300);\n",
        "SetDeaths(c.Foes, Exactly, 15, 0x12C);\n",
        "SetDeathsX(eud.AllPlayers, Exactly, 15, +300);\n",
    );
    let valid = concat!(
        "Deaths(Foes, Exactly, 15, \"Any unit\");\n",
        "Deaths(Foes, Exactly, 15, 229);\n",
        "Deaths(Foes, Exactly, 15, 0);\n",
        "Deaths(Foes, Exactly, 15, 232);\n",
        "Deaths(Player1, Exactly, 15, 300);\n",
        "Deaths(CurrentPlayer, Exactly, 15, 300);\n",
        "Deaths(player, Exactly, 15, 300);\n",
        "Deaths(Foe, Exactly, 15, 300);\n",
        "Deaths(Foes + 1, Exactly, 15, 300);\n",
        "Deaths(Foes, Exactly, 15, 232 + 1);\n",
        "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Anywhere\");\n",
    );

    for (uri, language_id) in [
        (TYPESCRIPT_URI, "typescript"),
        (TSX_URI, "typescriptreact"),
        (JAVASCRIPT_URI, "javascript"),
        (JSX_URI, "javascriptreact"),
    ] {
        client.open_document(uri, language_id, source);
        let diagnostics = client.diagnostics_for(uri);
        assert_eq!(diagnostics.len(), 4, "{language_id}");

        for (diagnostic, player) in
            diagnostics
                .iter()
                .zip(["Foes", "AllPlayers", "c.Foes", "eud.AllPlayers"])
        {
            assert_warning(diagnostic, "ambiguous");
            assert_warning(diagnostic, player.rsplit('.').next().unwrap());
            assert_warning(diagnostic, "Player1");

            let start = source.find(player).unwrap();
            assert_range(diagnostic, source, start, start + player.len());
        }

        client.change_document(uri, valid, 2);
        assert!(
            client.diagnostics_for(uri).is_empty(),
            "{language_id} must not warn about stock or ambiguous arguments"
        );

        client.close_document(uri);
        assert!(
            client.diagnostics_for(uri).is_empty(),
            "{language_id} should clear warnings on close"
        );
    }

    client.shutdown();
}
