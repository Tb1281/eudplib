//! Shared Map Symbol validation for Python and TypeScript/JavaScript.
//!
//! Only closed string literals that occupy a symbol-typed call argument are
//! validated. Numeric strings, canonical names, stock unit filters, and names
//! from the nearest Map Context are accepted. Everything else is reported as a
//! warning at the literal's source range.

use std::ops::Range;

use eudkit_core::canonical::stock_unit_id;
use eudkit_core::map_context::MapContext;

use crate::analysis::document::ParsedDocument;
use crate::analysis::symbol_slot::{symbol_slots, SymbolKind};
use crate::analysis::ts::{ts_symbol_slots, TsLanguage};

/// A Map Symbol warning at a byte range in the source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDiagnostic {
    pub range: Range<usize>,
    pub message: String,
}

/// Map Symbol warnings for a Python document.
pub fn python_diagnostics(source: &str, map_context: Option<&MapContext>) -> Vec<SymbolDiagnostic> {
    let document = ParsedDocument::parse(source);

    symbol_slots(&document)
        .into_iter()
        .filter_map(|slot| {
            let literal = slot.literal;
            let value = &source[literal.content_start..literal.content_end];
            validate_symbol(slot.kind, value, map_context).map(|message| SymbolDiagnostic {
                range: literal.literal_start..literal.literal_end,
                message,
            })
        })
        .collect()
}

/// Map Symbol warnings for a TypeScript or JavaScript document.
pub fn ts_diagnostics(
    source: &str,
    language: TsLanguage,
    map_context: Option<&MapContext>,
) -> Vec<SymbolDiagnostic> {
    ts_symbol_slots(source, language)
        .into_iter()
        .filter_map(|slot| {
            let range = slot.content_range();
            let value = &source[range.clone()];
            validate_symbol(slot.kind(), value, map_context).map(|message| SymbolDiagnostic {
                range: slot.literal_range(),
                message,
            })
        })
        .collect()
}

/// Validate one symbol-typed string value.
fn validate_symbol(
    kind: SymbolKind,
    value: &str,
    map_context: Option<&MapContext>,
) -> Option<String> {
    if is_numeric(value) {
        return None;
    }

    let resolved = match kind {
        SymbolKind::Unit => unit_id(value, map_context).is_some(),
        SymbolKind::Location => location_id(value, map_context).is_some(),
        SymbolKind::Switch => switch_id(value, map_context).is_some(),
    };

    (!resolved).then(|| unknown_message(kind, value))
}

fn unknown_message(kind: SymbolKind, value: &str) -> String {
    match kind {
        SymbolKind::Unit => format!(
            "Unknown unit \"{value}\". Use a canonical unit, a unit from the map, a numeric ID, or a stock unit filter."
        ),
        SymbolKind::Location => format!(
            "Unknown location \"{value}\". Use \"Anywhere\", \"Location N\", or a location from the map."
        ),
        SymbolKind::Switch => format!(
            "Unknown switch \"{value}\". Use \"Switch 1\" through \"Switch 256\" or a switch from the map."
        ),
    }
}

/// Numeric IDs and EUD offsets are valid symbol values.
fn is_numeric(value: &str) -> bool {
    value.trim().parse::<i64>().is_ok()
}

fn unit_id(value: &str, map_context: Option<&MapContext>) -> Option<u16> {
    match map_context {
        Some(context) => context.unit_id(value),
        None => stock_unit_id(value),
    }
}

fn location_id(value: &str, map_context: Option<&MapContext>) -> Option<u32> {
    if let Some(id) = map_context.and_then(|context| context.location_id(value)) {
        return Some(id);
    }

    canonical_location_id(value)
}

pub(crate) fn canonical_location_id(value: &str) -> Option<u32> {
    if value == "Anywhere" {
        return Some(64);
    }

    let index = value.strip_prefix("Location ")?.parse::<u32>().ok()?;
    ((1..=255).contains(&index) && index != 64).then_some(index)
}

fn switch_id(value: &str, map_context: Option<&MapContext>) -> Option<u8> {
    if let Some(id) = map_context.and_then(|context| context.switch_id(value)) {
        return Some(id);
    }

    canonical_switch_id(value)
}

pub(crate) fn canonical_switch_id(value: &str) -> Option<u8> {
    let index = value.strip_prefix("Switch ")?.parse::<u16>().ok()?;
    (1..=256).contains(&index).then_some((index - 1) as u8)
}

#[cfg(test)]
mod tests {
    use super::{python_diagnostics, ts_diagnostics};
    use crate::analysis::TsLanguage;

    fn python_messages(source: &str) -> Vec<String> {
        python_diagnostics(source, None)
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn accepts_canonical_units_filters_and_numeric_strings() {
        for value in [
            "Terran Marine",
            "Any unit",
            "Men",
            "Buildings",
            "Factories",
            "229",
            "300",
            "-1",
        ] {
            let source = format!("KillUnit(\"{value}\")");
            assert!(
                python_messages(&source).is_empty(),
                "{value:?} should not warn"
            );
        }
    }

    #[test]
    fn warns_about_unknown_units_locations_and_switches() {
        let source = concat!(
            "KillUnit(\"Not A Unit\")\n",
            "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Not A Location\")\n",
            "SetSwitch(\"Not A Switch\")\n",
        );
        let messages = python_messages(source);

        assert_eq!(messages.len(), 3);
        assert!(messages[0].contains("Unknown unit \"Not A Unit\""));
        assert!(messages[1].contains("Unknown location \"Not A Location\""));
        assert!(messages[2].contains("Unknown switch \"Not A Switch\""));
    }

    #[test]
    fn accepts_canonical_locations_and_switches() {
        for source in [
            "MinimapPing(\"Anywhere\")",
            "MinimapPing(\"Location 1\")",
            "MinimapPing(\"Location 255\")",
            "SetSwitch(\"Switch 1\")",
            "SetSwitch(\"Switch 256\")",
        ] {
            assert!(
                python_messages(source).is_empty(),
                "{source:?} should not warn"
            );
        }
    }

    #[test]
    fn location_64_is_anywhere_not_a_location_name() {
        for value in ["Location 0", "Location 64"] {
            let source = format!("MinimapPing(\"{value}\")");
            let messages = python_messages(&source);
            assert_eq!(messages.len(), 1, "{value:?} should warn");
            assert!(
                messages[0].contains("Unknown location"),
                "{value:?} should be reported as an unknown location"
            );
        }
    }

    #[test]
    fn ignores_free_form_strings_and_unclosed_literals() {
        for source in [
            "DisplayText(\"Not A Unit\")",
            "unit = \"Not A Unit\"",
            "KillUnit(\"Not A Unit",
            "KillUnit(f\"Not A Unit\")",
        ] {
            assert!(
                python_messages(source).is_empty(),
                "{source:?} should not warn"
            );
        }
    }

    #[test]
    fn warns_in_typescript_and_javascript() {
        for language in [
            TsLanguage::TypeScript,
            TsLanguage::Tsx,
            TsLanguage::JavaScript,
            TsLanguage::Jsx,
        ] {
            let source = "KillUnit(\"Not A Unit\"); SetSwitch(\"Not A Switch\");";
            let messages: Vec<_> = ts_diagnostics(source, language, None)
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect();

            assert_eq!(messages.len(), 2, "{language:?}");
            assert!(messages[0].contains("Unknown unit"));
            assert!(messages[1].contains("Unknown switch"));
        }
    }

    #[test]
    fn accepts_typescript_canonical_values() {
        let source = concat!(
            "KillUnit(\"Terran Marine\");\n",
            "KillUnit(\"Any unit\");\n",
            "KillUnit(\"300\");\n",
            "MinimapPing(\"Location 3\");\n",
            "SetSwitch(\"Switch 256\");\n",
        );

        assert!(ts_diagnostics(source, TsLanguage::TypeScript, None).is_empty());
    }
}
