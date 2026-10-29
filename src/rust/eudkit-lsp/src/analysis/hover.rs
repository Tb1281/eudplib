//! Markdown hover text for canonical symbols, map-context symbols, and calls.
//!
//! Hover is intentionally descriptive: it resolves the symbol that the editor
//! is pointing at and shows its ID or index. It does not type-check arbitrary
//! expressions; Pylance, tsserver, and basedpyright remain the source of
//! general type information.

use eudkit_core::canonical::CANONICAL_UNIT_NAMES;
use eudkit_core::map_context::MapSymbol;
use eudkit_core::symbols;

use crate::analysis::function_docs::function_doc;
use crate::analysis::SymbolKind;
use crate::analysis::{canonical_symbols, MapSymbolSources};

/// First canonical Stock Unit Filter ID (`Any unit`).
const STOCK_UNIT_FILTER_START: u16 = 229;
/// Last canonical Stock Unit Filter ID (`Factories`).
const STOCK_UNIT_FILTER_END: u16 = 232;

/// Markdown hover text for a symbol string, or `None` when it cannot be
/// resolved.
pub fn symbol_hover(
    kind: SymbolKind,
    value: &str,
    sources: Option<MapSymbolSources<'_>>,
) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.parse::<i64>().is_ok() {
        return None;
    }

    match kind {
        SymbolKind::Unit => unit_hover(value, sources),
        SymbolKind::Location => location_hover(value, sources),
        SymbolKind::Switch => switch_hover(value, sources),
    }
}

fn unit_hover(value: &str, sources: Option<MapSymbolSources<'_>>) -> Option<String> {
    let (id, map_id, stock_id) = match sources {
        Some((map, canonical)) => {
            let id = symbols::unit_id(map, canonical, value)?;
            let map_id = match map.unit_symbol(value) {
                MapSymbol::Unique(id) => Some(id),
                MapSymbol::Ambiguous | MapSymbol::Missing => None,
            };
            (id, map_id, canonical.unit_id(value))
        }
        None => {
            let id = canonical_symbols().unit_id(value)?;
            (id, None, Some(id))
        }
    };

    if (STOCK_UNIT_FILTER_START..=STOCK_UNIT_FILTER_END).contains(&id) {
        return Some(format!("**{value}**\n\nStock Unit Filter · ID {id}"));
    }

    let map_defined = match (map_id, stock_id) {
        (Some(map_id), Some(stock_id)) => map_id != stock_id,
        (Some(_), None) => true,
        _ => false,
    };
    let label = if map_defined {
        "Map Context unit"
    } else {
        "Canonical unit"
    };
    let mut text = format!("**{value}**\n\n{label} · ID {id}");

    if let Some(canonical) = CANONICAL_UNIT_NAMES.get(id as usize) {
        if *canonical != value {
            text.push_str(&format!("\n\nCanonical name: `{canonical}`"));
        }
    }

    Some(text)
}

fn location_hover(value: &str, sources: Option<MapSymbolSources<'_>>) -> Option<String> {
    let (id, map_location) = match sources {
        Some((map, canonical)) => {
            let id = symbols::location_id(map, canonical, value)?;
            let location = map
                .locations()
                .iter()
                .find(|location| location.id == id && location.name.as_deref() == Some(value));
            (id, location)
        }
        None => (canonical_symbols().location_id(value)?, None),
    };

    if let Some(location) = map_location {
        return Some(format!(
            "**{value}**\n\nMap Context location · ID {id}\n\nBounds: ({}, {}) – ({}, {})",
            location.left, location.top, location.right, location.bottom
        ));
    }

    Some(format!(
        "**{value}**\n\nCanonical location · ID {id} (`{value}`)"
    ))
}

fn switch_hover(value: &str, sources: Option<MapSymbolSources<'_>>) -> Option<String> {
    let (id, map_switch) = match sources {
        Some((map, canonical)) => {
            let id = symbols::switch_id(map, canonical, value)?;
            let switch = map
                .switches()
                .iter()
                .find(|switch| switch.id == id && switch.name.as_deref() == Some(value));
            (id, switch)
        }
        None => (canonical_symbols().switch_id(value)?, None),
    };
    let index = id as u32 + 1;

    let label = if map_switch.is_some() {
        "Map Context switch"
    } else {
        "Canonical switch"
    };
    Some(format!(
        "**{value}**\n\n{label} · ID {id} (0-based, `Switch {index}`)"
    ))
}

/// Markdown hover text for a supported call name.
pub fn function_hover(name: &str, language_id: &str) -> Option<String> {
    function_doc(name).map(|doc| doc.hover_markdown(language_id))
}

#[cfg(test)]
mod tests {
    use super::{function_hover, symbol_hover};
    use crate::analysis::SymbolKind;

    #[test]
    fn documents_canonical_units_and_filters() {
        let text = symbol_hover(SymbolKind::Unit, "Terran Marine", None).unwrap();
        assert!(text.contains("Canonical unit · ID 0"));

        let text = symbol_hover(SymbolKind::Unit, "Any unit", None).unwrap();
        assert!(text.contains("Stock Unit Filter · ID 229"));

        let text = symbol_hover(SymbolKind::Unit, "Jim Raynor (Vulture)", None).unwrap();
        assert!(text.contains("Canonical unit · ID 19"));
    }

    #[test]
    fn documents_canonical_locations_and_switches() {
        let text = symbol_hover(SymbolKind::Location, "Anywhere", None).unwrap();
        assert!(text.contains("Canonical location · ID 64"));

        let text = symbol_hover(SymbolKind::Location, "Location 1", None).unwrap();
        assert!(text.contains("Canonical location · ID 1"));

        let text = symbol_hover(SymbolKind::Switch, "Switch 1", None).unwrap();
        assert!(text.contains("Canonical switch · ID 0"));
        assert!(text.contains("`Switch 1`"));
    }

    #[test]
    fn rejects_unknown_numeric_and_invalid_symbols() {
        assert!(symbol_hover(SymbolKind::Unit, "Not A Unit", None).is_none());
        assert!(symbol_hover(SymbolKind::Location, "Location 64", None).is_none());
        assert!(symbol_hover(SymbolKind::Switch, "Switch 257", None).is_none());
        assert!(symbol_hover(SymbolKind::Unit, "229", None).is_none());
        assert!(symbol_hover(SymbolKind::Unit, "", None).is_none());
    }

    #[test]
    fn documents_supported_calls_per_language() {
        let text = function_hover("KillUnit", "python").unwrap();
        assert!(text.contains("**KillUnit(unit, player)**"));
        assert!(text.contains("Returns `ActionSpec`."));

        let text = function_hover("KillUnit", "typescript").unwrap();
        assert!(text.contains("Returns `Action`."));

        assert!(function_hover("DisplayText", "python").is_none());
    }
}
