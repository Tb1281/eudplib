//! Mapping a cursor position to the symbol slot it belongs to.
//!
//! A symbol slot is a place where a document-aware symbol name can be
//! completed. Only the `KillUnit` unit slot is implemented today; Map Context
//! slots can be added as further [`SymbolSlot`] variants without changing the
//! document or call-location layers.

use crate::analysis::call_site::enclosing_call;
use crate::analysis::document::{ParsedDocument, StringLiteral};

/// Function whose first argument is a unit symbol.
pub const UNIT_FUNCTION: &str = "KillUnit";
/// Keyword parameter for the unit symbol.
pub const UNIT_PARAMETER: &str = "unit";

/// The unit argument of a `KillUnit` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitNameSlot<'a> {
    pub literal: StringLiteral<'a>,
}

impl UnitNameSlot<'_> {
    /// Byte range of the existing literal content to replace on insertion.
    pub fn content_range(&self) -> std::ops::Range<usize> {
        self.literal.content_start..self.literal.content_end
    }
}

/// A document location that accepts a particular kind of symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolSlot<'a> {
    UnitName(UnitNameSlot<'a>),
}

/// Find the symbol slot containing `offset`, if any.
pub fn symbol_slot_at<'a>(document: &ParsedDocument<'a>, offset: usize) -> Option<SymbolSlot<'a>> {
    let literal = document.string_literal_at(offset)?;
    let call = enclosing_call(document.tokens(), document.source(), literal.token_index)?;

    if call.name != UNIT_FUNCTION || !call.argument_is_direct {
        return None;
    }

    let is_unit_slot = match call.keyword.as_deref() {
        Some(parameter) => parameter == UNIT_PARAMETER,
        None => call.argument_index == 0,
    };

    is_unit_slot.then_some(SymbolSlot::UnitName(UnitNameSlot { literal }))
}
