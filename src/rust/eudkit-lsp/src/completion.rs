//! Completion items derived from the reusable analysis layer.

use std::collections::HashSet;
use std::ops::Range;

use crate::analysis::{symbol_slot_at, ts_symbol_slot_at, ParsedDocument, SymbolKind, TsLanguage};
use eudkit_core::canonical::CANONICAL_UNIT_NAMES;
use eudkit_core::map_context::MapData;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Position, Range as LspRange, TextEdit,
};

/// Completion items for `offset` without a Map Context.
pub fn completions_at(document: &ParsedDocument<'_>, offset: usize) -> Option<Vec<CompletionItem>> {
    completions_at_with_context(document, offset, None)
}

/// Completion items for `offset`, using Map Symbols when a Map Context is
/// available and Canonical Symbols otherwise.
pub fn completions_at_with_context(
    document: &ParsedDocument<'_>,
    offset: usize,
    map_data: Option<&MapData>,
) -> Option<Vec<CompletionItem>> {
    let slot = symbol_slot_at(document, offset)?;
    Some(completion_items(
        document.source(),
        slot.content_range(),
        slot.kind,
        map_data,
    ))
}

/// Completion items for `offset` in a TypeScript or JavaScript document.
pub fn ts_completions_at(
    source: &str,
    language: TsLanguage,
    offset: usize,
    map_data: Option<&MapData>,
) -> Option<Vec<CompletionItem>> {
    let slot = ts_symbol_slot_at(source, language, offset)?;
    Some(completion_items(
        source,
        slot.content_range(),
        slot.kind(),
        map_data,
    ))
}

fn completion_items(
    source: &str,
    content_range: Range<usize>,
    kind: SymbolKind,
    map_data: Option<&MapData>,
) -> Vec<CompletionItem> {
    let range = byte_range_to_lsp(source, content_range.start, content_range.end);
    symbol_names(kind, map_data)
        .into_iter()
        .map(|name| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some(kind.detail().to_string()),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: name,
            })),
            ..CompletionItem::default()
        })
        .collect()
}

fn symbol_names(kind: SymbolKind, map_data: Option<&MapData>) -> Vec<String> {
    match kind {
        SymbolKind::Unit => canonical_unit_names(),
        SymbolKind::Location => location_names(map_data),
        SymbolKind::Switch => switch_names(map_data),
    }
}

impl SymbolKind {
    fn detail(self) -> &'static str {
        match self {
            SymbolKind::Unit => "EUD canonical unit",
            SymbolKind::Location => "EUD location",
            SymbolKind::Switch => "EUD switch",
        }
    }
}

fn canonical_unit_names() -> Vec<String> {
    CANONICAL_UNIT_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

fn location_names(map_data: Option<&MapData>) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    push_unique(&mut names, &mut seen, "Anywhere");

    if let Some(map_data) = map_data {
        for location in map_data.locations() {
            if let Some(name) = &location.name {
                push_unique(&mut names, &mut seen, name);
            }
        }
    }

    names
}

fn switch_names(map_data: Option<&MapData>) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();

    if let Some(map_data) = map_data {
        for switch in map_data.switches() {
            if let Some(name) = &switch.name {
                push_unique(&mut names, &mut seen, name);
            }
        }
    }

    for index in 1..=256 {
        push_unique(&mut names, &mut seen, &format!("Switch {index}"));
    }

    names
}

fn push_unique(names: &mut Vec<String>, seen: &mut HashSet<String>, name: &str) {
    if seen.insert(name.to_string()) {
        names.push(name.to_string());
    }
}

fn byte_range_to_lsp(source: &str, start: usize, end: usize) -> LspRange {
    LspRange::new(
        offset_to_position(source, start),
        offset_to_position(source, end),
    )
}

/// Convert a byte offset in `source` to an LSP position using UTF-16 columns.
pub fn offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    let character = source[line_start..offset].encode_utf16().count() as u32;
    Position::new(line, character)
}
