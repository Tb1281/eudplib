use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use serde_json::{json, Value};

#[test]
fn version_flag_prints_binary_name_and_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
        .arg("--version")
        .output()
        .expect("eudkit-lsp --version should run");

    assert!(
        output.status.success(),
        "eudkit-lsp --version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("eudkit-lsp {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_arguments_fail_with_usage_instead_of_starting_the_server() {
    let output = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
        .arg("--definitely-not-an-option")
        .output()
        .expect("eudkit-lsp should reject unknown arguments");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage: eudkit-lsp"), "{stderr}");
    assert!(stderr.contains("[--stdio]"), "{stderr}");
}

#[test]
fn stdio_flag_starts_a_full_language_server_session() {
    // vscode-languageclient passes `--stdio` for TransportKind.stdio. The
    // server must accept that flag and still complete initialize/shutdown.
    let mut session = StdioSession::spawn(&["--stdio"]);

    let initialize = session.request("initialize", json!({ "capabilities": {} }));
    assert!(
        initialize.get("result").is_some(),
        "initialize should return capabilities: {initialize}"
    );
    session.notify("initialized", json!({}));

    let shutdown = session.request("shutdown", Value::Null);
    assert!(
        shutdown.get("result").is_some_and(Value::is_null),
        "shutdown should return null: {shutdown}"
    );
    session.notify("exit", Value::Null);

    let stderr = session.wait_for_exit();
    assert!(stderr.is_empty(), "unexpected server output: {stderr}");
}

#[test]
fn server_invocations_exit_cleanly_when_stdin_closes() {
    for args in [&[][..], &["--stdio"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("eudkit-lsp should run with a closed stdin");

        assert!(
            output.status.success(),
            "eudkit-lsp {args:?} exited with {}; stderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "eudkit-lsp {args:?} wrote to stdout before initialize"
        );
    }
}

struct StdioSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    next_id: i64,
}

impl StdioSession {
    fn spawn(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
            .args(args)
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
        }
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

    fn wait_for_exit(mut self) -> String {
        drop(self.stdin);
        let status = self.child.wait().expect("eudkit-lsp should exit");
        let mut stderr = String::new();
        if let Some(mut stderr_pipe) = self.stderr.take() {
            stderr_pipe
                .read_to_string(&mut stderr)
                .expect("stderr should be readable");
        }
        assert!(status.success(), "eudkit-lsp exited with {status}");
        stderr
    }
}
