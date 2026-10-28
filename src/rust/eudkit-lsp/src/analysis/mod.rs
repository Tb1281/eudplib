//! Reusable Python and TypeScript analysis for eudkit language-server features.

pub mod call_site;
pub mod document;
pub mod eud_player;
pub mod symbol_slot;
pub mod ts;
pub mod validation;

pub use call_site::{enclosing_call, CallSite};
pub use document::{ParsedDocument, StringLiteral};
pub use eud_player::{
    eud_player_warning, python_eud_player_diagnostics, ts_eud_player_diagnostics,
    EudPlayerDiagnostic,
};
pub use symbol_slot::{
    function_symbols, symbol_slot_at, FunctionSymbols, SymbolArgument, SymbolKind, SymbolSlot,
    PYTHON_SYMBOLS,
};
pub use ts::{ts_symbol_slot_at, TsLanguage, TsSymbolSlot};
pub use validation::{python_diagnostics, ts_diagnostics, SymbolDiagnostic};
