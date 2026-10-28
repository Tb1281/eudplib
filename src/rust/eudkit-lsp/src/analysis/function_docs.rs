//! Hover and signature-help documentation for the supported frontend calls.
//!
//! The symbol table in [`crate::analysis::symbol_slot`] decides which arguments
//! accept canonical or map-context symbols. This table adds the call shape and
//! a one-line summary used by `textDocument/hover` and
//! `textDocument/signatureHelp`. Keep the two tables in sync; the tests below
//! fail when a symbol-table function has no entry here.

/// Return type shown when a call is documented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionResult {
    Condition,
    Action,
}

impl FunctionResult {
    /// Python return type.
    pub fn python_name(self) -> &'static str {
        match self {
            Self::Condition => "Condition",
            Self::Action => "ActionSpec",
        }
    }

    /// TypeScript/JavaScript return type.
    pub fn typescript_name(self) -> &'static str {
        match self {
            Self::Condition => "Condition",
            Self::Action => "Action",
        }
    }

    /// Return type for a language id reported by the editor.
    pub fn name_for_language(self, language_id: &str) -> &'static str {
        if language_id == "python" {
            self.python_name()
        } else {
            self.typescript_name()
        }
    }
}

/// Hover/signature documentation for one supported call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionDoc {
    pub name: &'static str,
    /// Parameter names in call order.
    pub parameters: &'static [&'static str],
    pub result: FunctionResult,
    /// One-line summary shown above the parameter notes.
    pub summary: &'static str,
}

impl FunctionDoc {
    /// `Name(param, param, ...)` without language-specific types.
    pub fn signature(&self) -> String {
        format!("{}({})", self.name, self.parameters.join(", "))
    }

    /// Markdown hover text for this call.
    pub fn hover_markdown(&self, language_id: &str) -> String {
        let mut text = format!(
            "**{}**\n\n{}\n\nReturns `{}`.",
            self.signature(),
            self.summary,
            self.result.name_for_language(language_id)
        );

        if let Some(function) = crate::analysis::symbol_slot::function_symbols(self.name) {
            for argument in function.arguments {
                let Some(parameter) = self.parameters.get(argument.index) else {
                    continue;
                };
                let accepts = match argument.kind {
                    crate::analysis::SymbolKind::Unit => {
                        "a canonical unit name, a map-context unit name, or a numeric unit ID"
                    }
                    crate::analysis::SymbolKind::Location => {
                        "a canonical location (`Anywhere`, `Location N`), a map-context location name, or a numeric location ID"
                    }
                    crate::analysis::SymbolKind::Switch => {
                        "a canonical switch (`Switch 1` through `Switch 256`), a map-context switch name, or a numeric switch ID"
                    }
                };
                text.push_str(&format!("\n\n`{parameter}` accepts {accepts}."));
            }
        }

        text
    }
}

/// All calls with hover and signature documentation.
pub const FUNCTION_DOCS: &[FunctionDoc] = &[
    FunctionDoc {
        name: "KillUnit",
        parameters: &["unit", "player"],
        result: FunctionResult::Action,
        summary: "Kills all units of a type owned by a player.",
    },
    FunctionDoc {
        name: "Deaths",
        parameters: &["player", "comparison", "amount", "unit"],
        result: FunctionResult::Condition,
        summary: "Checks a player's death counter for one unit type.",
    },
    FunctionDoc {
        name: "DeathsX",
        parameters: &["player", "comparison", "amount", "unit", "mask"],
        result: FunctionResult::Condition,
        summary: "Masked variant of `Deaths` using EUD extended death counters.",
    },
    FunctionDoc {
        name: "SetDeaths",
        parameters: &["player", "modifier", "value", "unit"],
        result: FunctionResult::Action,
        summary: "Modifies a player's death counter for one unit type.",
    },
    FunctionDoc {
        name: "SetDeathsX",
        parameters: &["player", "modifier", "value", "unit", "mask"],
        result: FunctionResult::Action,
        summary: "Masked variant of `SetDeaths` using EUD extended death counters.",
    },
    FunctionDoc {
        name: "Bring",
        parameters: &["player", "comparison", "amount", "unit", "location"],
        result: FunctionResult::Condition,
        summary: "Checks units a player brings to a location.",
    },
    FunctionDoc {
        name: "CreateUnit",
        parameters: &["count", "unit", "location", "player"],
        result: FunctionResult::Action,
        summary: "Creates units of a type at a location for a player.",
    },
    FunctionDoc {
        name: "KillUnitAt",
        parameters: &["count", "unit", "location", "player"],
        result: FunctionResult::Action,
        summary: "Kills up to a count of units of a type at a location.",
    },
    FunctionDoc {
        name: "RemoveUnitAt",
        parameters: &["count", "unit", "location", "player"],
        result: FunctionResult::Action,
        summary: "Removes up to a count of units of a type at a location.",
    },
    FunctionDoc {
        name: "MinimapPing",
        parameters: &["location"],
        result: FunctionResult::Action,
        summary: "Pings the minimap at a location for all players.",
    },
    FunctionDoc {
        name: "MoveLocation",
        parameters: &["location", "unit", "player", "dest_location"],
        result: FunctionResult::Action,
        summary: "Centers a location on units of a type owned by a player.",
    },
    FunctionDoc {
        name: "MoveUnit",
        parameters: &["count", "unit", "player", "start_location", "dest_location"],
        result: FunctionResult::Action,
        summary: "Teleports up to a count of units between locations.",
    },
    FunctionDoc {
        name: "GiveUnits",
        parameters: &["count", "unit", "player", "location", "new_player"],
        result: FunctionResult::Action,
        summary: "Transfers units from one player to another.",
    },
    FunctionDoc {
        name: "Order",
        parameters: &[
            "unit",
            "player",
            "start_location",
            "order_type",
            "dest_location",
        ],
        result: FunctionResult::Action,
        summary: "Orders units at a location to move, patrol, or attack.",
    },
    FunctionDoc {
        name: "Switch",
        parameters: &["switch", "state"],
        result: FunctionResult::Condition,
        summary: "Checks a switch's state.",
    },
    FunctionDoc {
        name: "SetSwitch",
        parameters: &["switch", "state"],
        result: FunctionResult::Action,
        summary: "Changes a switch's state.",
    },
];

/// Find documentation for a supported call.
pub fn function_doc(name: &str) -> Option<&'static FunctionDoc> {
    FUNCTION_DOCS.iter().find(|doc| doc.name == name)
}

#[cfg(test)]
mod tests {
    use super::{function_doc, FUNCTION_DOCS};
    use crate::analysis::symbol_slot::PYTHON_SYMBOLS;

    #[test]
    fn every_symbol_table_function_has_documentation() {
        for function in PYTHON_SYMBOLS {
            let doc = function_doc(function.name)
                .unwrap_or_else(|| panic!("missing documentation for {}", function.name));
            for argument in function.arguments {
                assert!(
                    doc.parameters.get(argument.index).is_some(),
                    "{} has no parameter at index {}",
                    function.name,
                    argument.index
                );
            }
        }
    }

    #[test]
    fn every_documented_function_is_in_the_symbol_table() {
        for doc in FUNCTION_DOCS {
            assert!(
                PYTHON_SYMBOLS
                    .iter()
                    .any(|function| function.name == doc.name),
                "{} is documented but not in the symbol table",
                doc.name
            );
        }
    }

    #[test]
    fn hover_text_includes_signature_summary_and_result() {
        let text = function_doc("KillUnit").unwrap().hover_markdown("python");
        assert!(text.contains("**KillUnit(unit, player)**"));
        assert!(text.contains("Kills all units of a type owned by a player."));
        assert!(text.contains("Returns `ActionSpec`."));
        assert!(text.contains("`unit` accepts a canonical unit name"));

        let text = function_doc("Deaths").unwrap().hover_markdown("typescript");
        assert!(text.contains("Returns `Condition`."));
    }
}
