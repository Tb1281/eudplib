//! Completion items derived from the reusable analysis layer.

use crate::analysis::{symbol_slot_at, ParsedDocument, SymbolSlot};
use eudkit_core::map_context::CANONICAL_UNIT_NAMES;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Position, Range, TextEdit,
};

/// Completion items for `offset`, or `None` when the cursor is not in a known
/// symbol slot.
pub fn completions_at(document: &ParsedDocument<'_>, offset: usize) -> Option<Vec<CompletionItem>> {
    match symbol_slot_at(document, offset)? {
        SymbolSlot::UnitName(slot) => {
            let range = byte_range_to_lsp(
                document.source(),
                slot.content_range().start,
                slot.content_range().end,
            );
            Some(
                CANONICAL_UNIT_NAMES
                    .iter()
                    .map(|name| CompletionItem {
                        label: (*name).to_string(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        detail: Some("EUD canonical unit".to_string()),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range,
                            new_text: (*name).to_string(),
                        })),
                        ..CompletionItem::default()
                    })
                    .collect(),
            )
        }
    }
}

fn byte_range_to_lsp(source: &str, start: usize, end: usize) -> Range {
    Range::new(
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
