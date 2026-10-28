//! The synchronous stdio language server.

use std::collections::HashMap;
use std::error::Error;
use std::ops::Range;

use crate::analysis::{
    python_diagnostics, python_eud_player_diagnostics, ts_diagnostics, ts_eud_player_diagnostics,
    ParsedDocument, TsLanguage,
};
use crate::completion::{completions_at_with_context, offset_to_position, ts_completions_at};
use crate::project::ProjectCache;
use eudkit_core::map_context::MapContext;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, Position, PublishDiagnosticsParams, Range as LspRange, ServerCapabilities,
    TextDocumentSyncKind, Uri,
};
use url::Url;

type ServerResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone)]
struct Document {
    text: String,
    language_id: String,
}

#[derive(Default)]
struct ServerState {
    documents: HashMap<String, Document>,
    projects: ProjectCache,
}

/// Run the language server over stdin/stdout.
pub fn run() -> ServerResult<()> {
    let (connection, io_threads) = Connection::stdio();
    run_with_connection(connection)?;
    io_threads.join()?;
    Ok(())
}

fn run_with_connection(connection: Connection) -> ServerResult<()> {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncKind::FULL.into()),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["\"".to_string(), "'".to_string(), "(".to_string()]),
            ..CompletionOptions::default()
        }),
        ..ServerCapabilities::default()
    };
    let initialize_result = serde_json::to_value(capabilities)?;
    let initialize_params = connection.initialize(initialize_result)?;
    let _: InitializeParams = serde_json::from_value(initialize_params)?;

    main_loop(connection)
}

fn main_loop(connection: Connection) -> ServerResult<()> {
    let mut state = ServerState::default();

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                let response = handle_request(&request, &mut state);
                if connection.sender.send(response.into()).is_err() {
                    break;
                }
            }
            Message::Notification(notification) => {
                handle_notification(
                    &connection,
                    &notification.method,
                    notification.params.clone(),
                    &mut state,
                );
            }
            Message::Response(_) => {}
        }
    }

    Ok(())
}

fn handle_request(request: &Request, state: &mut ServerState) -> Response {
    match request.method.as_str() {
        "textDocument/completion" => {
            match serde_json::from_value::<CompletionParams>(request.params.clone()) {
                Ok(params) => {
                    Response::new_ok(request.id.clone(), completion_response(state, &params))
                }
                Err(error) => Response::new_err(
                    request.id.clone(),
                    ErrorCode::InvalidParams as i32,
                    error.to_string(),
                ),
            }
        }
        _ => Response::new_err(
            request.id.clone(),
            ErrorCode::MethodNotFound as i32,
            format!("unsupported request: {}", request.method),
        ),
    }
}

fn completion_response(
    state: &mut ServerState,
    params: &CompletionParams,
) -> Option<CompletionResponse> {
    let position = params.text_document_position.position;
    let uri = params.text_document_position.text_document.uri.as_str();
    let document = state.documents.get(uri)?;
    let offset = position_to_offset(&document.text, position)?;
    let map_context = Url::parse(uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|path| state.projects.map_context_for_document(&path));
    let items = match document.language_id.as_str() {
        "python" => {
            let parsed = ParsedDocument::parse(&document.text);
            completions_at_with_context(&parsed, offset, map_context.as_deref())
        }
        language_id => {
            let language = TsLanguage::from_language_id(language_id)?;
            ts_completions_at(&document.text, language, offset, map_context.as_deref())
        }
    }?;
    Some(CompletionResponse::Array(items))
}

fn handle_notification(
    connection: &Connection,
    method: &str,
    params: serde_json::Value,
    state: &mut ServerState,
) {
    match method {
        "textDocument/didOpen" => {
            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
                let uri = params.text_document.uri.as_str().to_string();
                state.documents.insert(
                    uri.clone(),
                    Document {
                        text: params.text_document.text,
                        language_id: params.text_document.language_id,
                    },
                );
                publish_diagnostics(connection, state, &uri);
            }
        }
        "textDocument/didChange" => {
            if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                let uri = params.text_document.uri.as_str().to_string();
                if let Some(change) = params.content_changes.into_iter().next_back() {
                    if let Some(document) = state.documents.get_mut(&uri) {
                        document.text = change.text;
                    }
                }
                publish_diagnostics(connection, state, &uri);
            }
        }
        "textDocument/didClose" => {
            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                let uri = params.text_document.uri.as_str().to_string();
                state.documents.remove(&uri);
                send_diagnostics(connection, &uri, Vec::new());
            }
        }
        _ => {}
    }
}

/// Validate the open document and publish its Map Symbol warnings.
fn publish_diagnostics(connection: &Connection, state: &mut ServerState, uri: &str) {
    let ServerState {
        documents,
        projects,
    } = state;
    let Some(document) = documents.get(uri) else {
        return;
    };
    let map_context = Url::parse(uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|path| projects.map_context_for_document(&path));
    let diagnostics = document_diagnostics(document, map_context.as_deref());
    send_diagnostics(connection, uri, diagnostics);
}

fn document_diagnostics(document: &Document, map_context: Option<&MapContext>) -> Vec<Diagnostic> {
    let mut findings: Vec<(Range<usize>, String)> = Vec::new();

    match document.language_id.as_str() {
        "python" => {
            findings.extend(
                python_diagnostics(&document.text, map_context)
                    .into_iter()
                    .map(|diagnostic| (diagnostic.range, diagnostic.message)),
            );
            findings.extend(
                python_eud_player_diagnostics(&document.text)
                    .into_iter()
                    .map(|diagnostic| (diagnostic.range, diagnostic.message)),
            );
        }
        language_id => match TsLanguage::from_language_id(language_id) {
            Some(language) => {
                findings.extend(
                    ts_diagnostics(&document.text, language, map_context)
                        .into_iter()
                        .map(|diagnostic| (diagnostic.range, diagnostic.message)),
                );
                findings.extend(
                    ts_eud_player_diagnostics(&document.text, language)
                        .into_iter()
                        .map(|diagnostic| (diagnostic.range, diagnostic.message)),
                );
            }
            None => return Vec::new(),
        },
    }

    findings.sort_by_key(|(range, _)| range.start);

    findings
        .into_iter()
        .map(|(range, message)| Diagnostic {
            range: LspRange::new(
                offset_to_position(&document.text, range.start),
                offset_to_position(&document.text, range.end),
            ),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("eudkit".to_string()),
            message,
            ..Diagnostic::default()
        })
        .collect()
}

fn send_diagnostics(connection: &Connection, uri: &str, diagnostics: Vec<Diagnostic>) {
    let Ok(uri) = uri.parse::<Uri>() else {
        return;
    };
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    let _ = connection
        .sender
        .send(Message::Notification(Notification::new(
            "textDocument/publishDiagnostics".to_string(),
            params,
        )));
}

/// Convert an LSP UTF-16 position to a byte offset in `source`.
fn position_to_offset(source: &str, position: Position) -> Option<usize> {
    let mut line = 0u32;
    let mut offset = 0usize;

    while line < position.line {
        let newline = source[offset..].find('\n')?;
        offset += newline + 1;
        line += 1;
    }

    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |newline| offset + newline);
    let mut utf16_column = 0u32;

    for (byte_index, character) in source[offset..line_end].char_indices() {
        if utf16_column >= position.character {
            return Some(offset + byte_index);
        }
        utf16_column += character.len_utf16() as u32;
        if utf16_column > position.character {
            return None;
        }
    }

    Some(line_end)
}

#[cfg(test)]
mod tests {
    use super::position_to_offset;
    use lsp_types::Position;

    #[test]
    fn converts_utf16_positions_to_byte_offsets() {
        let source = "a😀b\nKillUnit(\"Mar";
        assert_eq!(position_to_offset(source, Position::new(0, 0)), Some(0));
        assert_eq!(position_to_offset(source, Position::new(0, 1)), Some(1));
        assert_eq!(position_to_offset(source, Position::new(0, 3)), Some(5));
        assert_eq!(position_to_offset(source, Position::new(0, 4)), Some(6));
        assert_eq!(
            position_to_offset(source, Position::new(1, 13)),
            Some(source.len())
        );
    }
}
