//! Reusable Python and TypeScript analysis for eudkit language-server features.

pub mod call_site;
pub mod document;
pub mod symbol_slot;
pub mod ts;

pub use call_site::{enclosing_call, CallSite};
pub use document::{ParsedDocument, StringLiteral};
pub use symbol_slot::{symbol_slot_at, SymbolSlot, UnitNameSlot};
pub use ts::{ts_symbol_slot_at, TsLanguage, TsUnitNameSlot};
