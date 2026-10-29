//! Reusable Python and TypeScript analysis for eudkit language-server features.

use std::sync::LazyLock;

use eudkit_core::canonical::CanonicalSymbols;
use eudkit_core::map_context::MapData;

pub mod call_site;
pub mod document;
pub mod eud_player;
pub mod function_docs;
pub mod hover;
pub mod signature;
pub mod symbol_slot;
pub mod ts;
pub mod validation;

/// Borrowed Map Data and Canonical Symbols for one analysis pass.
pub type MapSymbolSources<'a> = (&'a MapData, &'a CanonicalSymbols);

/// Canonical Symbols used when a document has no Map Context.
pub(crate) fn canonical_symbols() -> &'static CanonicalSymbols {
    static CANONICAL_SYMBOLS: LazyLock<CanonicalSymbols> = LazyLock::new(CanonicalSymbols::new);
    &CANONICAL_SYMBOLS
}

pub use call_site::{enclosing_call, CallSite};
pub use document::{ParsedDocument, StringLiteral};
pub use eud_player::{
    eud_player_warning, python_eud_player_diagnostics, ts_eud_player_diagnostics,
    EudPlayerDiagnostic,
};
pub use function_docs::{function_doc, FunctionDoc, FunctionResult, FUNCTION_DOCS};
pub use hover::{function_hover, symbol_hover};
pub use signature::{identifier_at, python_call_context, ts_call_context, CallContext};
pub use symbol_slot::{
    function_symbols, symbol_slot_at, FunctionSymbols, SymbolArgument, SymbolKind, SymbolSlot,
    PYTHON_SYMBOLS,
};
pub use ts::{ts_symbol_slot_at, TsLanguage, TsSymbolSlot};
pub use validation::{python_diagnostics, ts_diagnostics, SymbolDiagnostic};
