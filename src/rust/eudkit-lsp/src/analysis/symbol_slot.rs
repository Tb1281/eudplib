//! Mapping a cursor position to the symbol slot it belongs to.
//!
//! The symbol table is shared by frontend analyzers. Python is the only
//! frontend today; the table can be reused (or extended with another table)
//! when the TypeScript/JavaScript analyzer needs the same mappings.

use crate::analysis::call_site::enclosing_call;
use crate::analysis::document::{ParsedDocument, StringLiteral};

/// Frontend function whose first argument is a unit symbol.
pub const UNIT_FUNCTION: &str = "KillUnit";

/// A kind of named symbol accepted by a frontend API argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Unit,
    Location,
    Switch,
}

/// One function argument that accepts a symbol name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolArgument {
    pub kind: SymbolKind,
    /// Zero-based positional argument index.
    pub index: usize,
    /// Keyword parameter names accepted for this argument.
    pub keywords: &'static [&'static str],
}

impl SymbolArgument {
    const fn new(kind: SymbolKind, index: usize, keywords: &'static [&'static str]) -> Self {
        Self {
            kind,
            index,
            keywords,
        }
    }

    /// Whether this argument matches a call's argument position or keyword.
    pub fn matches(&self, argument_index: usize, keyword: Option<&str>) -> bool {
        match keyword {
            Some(keyword) => self.keywords.contains(&keyword),
            None => self.index == argument_index,
        }
    }
}

/// Symbol-typed arguments of one frontend function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionSymbols {
    pub name: &'static str,
    pub arguments: &'static [SymbolArgument],
}

/// Python functions that accept Map or Canonical Symbols by name.
pub const PYTHON_SYMBOLS: &[FunctionSymbols] = &[
    FunctionSymbols {
        name: UNIT_FUNCTION,
        arguments: &[SymbolArgument::new(SymbolKind::Unit, 0, &["unit"])],
    },
    FunctionSymbols {
        name: "Deaths",
        arguments: &[SymbolArgument::new(SymbolKind::Unit, 3, &["unit"])],
    },
    FunctionSymbols {
        name: "DeathsX",
        arguments: &[SymbolArgument::new(SymbolKind::Unit, 3, &["unit"])],
    },
    FunctionSymbols {
        name: "SetDeaths",
        arguments: &[SymbolArgument::new(SymbolKind::Unit, 3, &["unit"])],
    },
    FunctionSymbols {
        name: "SetDeathsX",
        arguments: &[SymbolArgument::new(SymbolKind::Unit, 3, &["unit"])],
    },
    FunctionSymbols {
        name: "Bring",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 3, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 4, &["location"]),
        ],
    },
    FunctionSymbols {
        name: "CreateUnit",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 1, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 2, &["location"]),
        ],
    },
    FunctionSymbols {
        name: "KillUnitAt",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 1, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 2, &["location"]),
        ],
    },
    FunctionSymbols {
        name: "RemoveUnitAt",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 1, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 2, &["location"]),
        ],
    },
    FunctionSymbols {
        name: "MinimapPing",
        arguments: &[SymbolArgument::new(SymbolKind::Location, 0, &["location"])],
    },
    FunctionSymbols {
        name: "MoveLocation",
        arguments: &[
            SymbolArgument::new(SymbolKind::Location, 0, &["location"]),
            SymbolArgument::new(SymbolKind::Unit, 1, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 3, &["dest_location"]),
        ],
    },
    FunctionSymbols {
        name: "MoveUnit",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 1, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 3, &["start_location"]),
            SymbolArgument::new(SymbolKind::Location, 4, &["dest_location"]),
        ],
    },
    FunctionSymbols {
        name: "GiveUnits",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 1, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 3, &["location"]),
        ],
    },
    FunctionSymbols {
        name: "Order",
        arguments: &[
            SymbolArgument::new(SymbolKind::Unit, 0, &["unit"]),
            SymbolArgument::new(SymbolKind::Location, 2, &["start_location"]),
            SymbolArgument::new(SymbolKind::Location, 4, &["dest_location"]),
        ],
    },
    FunctionSymbols {
        name: "Switch",
        arguments: &[SymbolArgument::new(SymbolKind::Switch, 0, &["switch"])],
    },
    FunctionSymbols {
        name: "SetSwitch",
        arguments: &[SymbolArgument::new(SymbolKind::Switch, 0, &["switch"])],
    },
];

/// Find the symbol table entry for a frontend function name.
pub fn function_symbols(name: &str) -> Option<&'static FunctionSymbols> {
    PYTHON_SYMBOLS.iter().find(|function| function.name == name)
}

/// A document location that accepts a particular kind of symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolSlot<'a> {
    pub kind: SymbolKind,
    pub literal: StringLiteral<'a>,
}

impl SymbolSlot<'_> {
    /// Byte range of the existing literal content to replace on insertion.
    pub fn content_range(&self) -> std::ops::Range<usize> {
        self.literal.content_start..self.literal.content_end
    }
}

/// Find the symbol slot containing `offset`, if any.
pub fn symbol_slot_at<'a>(document: &ParsedDocument<'a>, offset: usize) -> Option<SymbolSlot<'a>> {
    let literal = document.string_literal_at(offset)?;
    let call = enclosing_call(document.tokens(), document.source(), literal.token_index)?;

    if !call.argument_is_direct {
        return None;
    }

    let function = function_symbols(&call.name)?;
    let argument = function
        .arguments
        .iter()
        .find(|argument| argument.matches(call.argument_index, call.keyword.as_deref()))?;

    Some(SymbolSlot {
        kind: argument.kind,
        literal,
    })
}

#[cfg(test)]
mod tests {
    use super::{function_symbols, symbol_slot_at, SymbolKind, PYTHON_SYMBOLS};
    use crate::analysis::ParsedDocument;

    fn kind_at(source: &str) -> Option<SymbolKind> {
        symbol_slot_at(&ParsedDocument::parse(source), source.len()).map(|slot| slot.kind)
    }

    #[test]
    fn maps_unit_arguments_for_python_functions() {
        for (function, index) in [
            ("KillUnit", 0),
            ("Deaths", 3),
            ("DeathsX", 3),
            ("SetDeaths", 3),
            ("SetDeathsX", 3),
            ("Bring", 3),
            ("CreateUnit", 1),
            ("KillUnitAt", 1),
            ("RemoveUnitAt", 1),
            ("MoveLocation", 1),
            ("MoveUnit", 1),
            ("GiveUnits", 1),
            ("Order", 0),
        ] {
            let arguments = "0, ".repeat(index);
            let source = format!("{function}({arguments}\"");
            assert_eq!(kind_at(&source), Some(SymbolKind::Unit), "{source}");
        }
    }

    #[test]
    fn maps_location_and_switch_arguments_for_python_functions() {
        for (function, index, expected) in [
            ("Bring", 4, SymbolKind::Location),
            ("CreateUnit", 2, SymbolKind::Location),
            ("KillUnitAt", 2, SymbolKind::Location),
            ("RemoveUnitAt", 2, SymbolKind::Location),
            ("MinimapPing", 0, SymbolKind::Location),
            ("MoveLocation", 0, SymbolKind::Location),
            ("MoveLocation", 3, SymbolKind::Location),
            ("MoveUnit", 3, SymbolKind::Location),
            ("MoveUnit", 4, SymbolKind::Location),
            ("GiveUnits", 3, SymbolKind::Location),
            ("Order", 2, SymbolKind::Location),
            ("Order", 4, SymbolKind::Location),
            ("Switch", 0, SymbolKind::Switch),
            ("SetSwitch", 0, SymbolKind::Switch),
        ] {
            let arguments = "0, ".repeat(index);
            let source = format!("{function}({arguments}\"");
            assert_eq!(kind_at(&source), Some(expected), "{source}");
        }
    }

    #[test]
    fn accepts_keyword_symbol_arguments() {
        assert_eq!(
            kind_at("KillUnit(player=1, unit=\""),
            Some(SymbolKind::Unit)
        );
        assert_eq!(kind_at("Deaths(1, 2, 3, unit=\""), Some(SymbolKind::Unit));
        assert_eq!(
            kind_at("MoveLocation(dest_location=\""),
            Some(SymbolKind::Location)
        );
        assert_eq!(kind_at("SetSwitch(switch=\""), Some(SymbolKind::Switch));
    }

    #[test]
    fn rejects_non_symbol_arguments_and_unsupported_functions() {
        assert_eq!(kind_at("KillUnit(\"Terran Marine\", \""), None);
        assert_eq!(kind_at("KillUnit(1, \""), None);
        assert_eq!(kind_at("DisplayText(\""), None);
        assert_eq!(kind_at("KillUnit(unit=\"x\" + \""), None);
    }

    #[test]
    fn table_entries_are_unique_and_cover_required_functions() {
        let mut names: Vec<&str> = PYTHON_SYMBOLS.iter().map(|entry| entry.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PYTHON_SYMBOLS.len());

        for name in [
            "KillUnit",
            "Deaths",
            "DeathsX",
            "SetDeaths",
            "SetDeathsX",
            "Bring",
            "CreateUnit",
            "KillUnitAt",
            "RemoveUnitAt",
            "MinimapPing",
            "MoveLocation",
            "MoveUnit",
            "GiveUnits",
            "Order",
            "Switch",
            "SetSwitch",
        ] {
            assert!(function_symbols(name).is_some(), "missing {name}");
        }
    }
}
