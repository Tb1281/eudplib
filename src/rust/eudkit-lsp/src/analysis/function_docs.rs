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
    /// Python parameter names in call order.
    pub parameters: &'static [&'static str],
    /// TypeScript/JavaScript parameter names in call order.
    pub typescript_parameters: &'static [&'static str],
    pub result: FunctionResult,
    /// One-line summary shown above the parameter notes.
    pub summary: &'static str,
}

impl FunctionDoc {
    /// Parameter names for a language id reported by the editor.
    pub fn parameters_for(&self, language_id: &str) -> &'static [&'static str] {
        if language_id == "python" {
            self.parameters
        } else {
            self.typescript_parameters
        }
    }

    /// `Name(param, param, ...)` without language-specific types.
    pub fn signature(&self, language_id: &str) -> String {
        format!(
            "{}({})",
            self.name,
            self.parameters_for(language_id).join(", ")
        )
    }

    /// `Returns \`Type\`.` line shared by hover and signature documentation.
    pub fn returns_markdown(&self, language_id: &str) -> String {
        format!("Returns `{}`.", self.result.name_for_language(language_id))
    }

    /// Markdown hover text for this call.
    pub fn hover_markdown(&self, language_id: &str) -> String {
        let mut text = format!(
            "**{}**\n\n{}\n\n{}",
            self.signature(language_id),
            self.summary,
            self.returns_markdown(language_id)
        );

        if let Some(function) = crate::analysis::symbol_slot::function_symbols(self.name) {
            for argument in function.arguments {
                let Some(parameter) = self.parameters_for(language_id).get(argument.index) else {
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
        typescript_parameters: &["unit", "player"],
        result: FunctionResult::Action,
        summary: "Kills all units of a type owned by a player.",
    },
    FunctionDoc {
        name: "Deaths",
        parameters: &["player", "comparison", "amount", "unit"],
        typescript_parameters: &["player", "comparison", "amount", "unit"],
        result: FunctionResult::Condition,
        summary: "Checks a player's death counter for one unit type.",
    },
    FunctionDoc {
        name: "DeathsX",
        parameters: &["player", "comparison", "amount", "unit", "mask"],
        typescript_parameters: &["player", "comparison", "amount", "unit", "mask"],
        result: FunctionResult::Condition,
        summary: "Masked variant of `Deaths` using EUD extended death counters.",
    },
    FunctionDoc {
        name: "SetDeaths",
        parameters: &["player", "modifier", "value", "unit"],
        typescript_parameters: &["player", "modifier", "value", "unit"],
        result: FunctionResult::Action,
        summary: "Modifies a player's death counter for one unit type.",
    },
    FunctionDoc {
        name: "SetDeathsX",
        parameters: &["player", "modifier", "value", "unit", "mask"],
        typescript_parameters: &["player", "modifier", "value", "unit", "mask"],
        result: FunctionResult::Action,
        summary: "Masked variant of `SetDeaths` using EUD extended death counters.",
    },
    FunctionDoc {
        name: "Bring",
        parameters: &["player", "comparison", "amount", "unit", "location"],
        typescript_parameters: &["player", "comparison", "amount", "unit", "location"],
        result: FunctionResult::Condition,
        summary: "Checks units a player brings to a location.",
    },
    FunctionDoc {
        name: "CreateUnit",
        parameters: &["count", "unit", "location", "player"],
        typescript_parameters: &["count", "unit", "location", "player"],
        result: FunctionResult::Action,
        summary: "Creates units of a type at a location for a player.",
    },
    FunctionDoc {
        name: "KillUnitAt",
        parameters: &["count", "unit", "location", "player"],
        typescript_parameters: &["count", "unit", "location", "player"],
        result: FunctionResult::Action,
        summary: "Kills up to a count of units of a type at a location.",
    },
    FunctionDoc {
        name: "RemoveUnitAt",
        parameters: &["count", "unit", "location", "player"],
        typescript_parameters: &["count", "unit", "location", "player"],
        result: FunctionResult::Action,
        summary: "Removes up to a count of units of a type at a location.",
    },
    FunctionDoc {
        name: "MinimapPing",
        parameters: &["location"],
        typescript_parameters: &["location"],
        result: FunctionResult::Action,
        summary: "Pings the minimap at a location for all players.",
    },
    FunctionDoc {
        name: "MoveLocation",
        parameters: &["location", "unit", "player", "dest_location"],
        typescript_parameters: &["location", "onUnit", "owner", "destLocation"],
        result: FunctionResult::Action,
        summary: "Centers a location on units of a type owned by a player.",
    },
    FunctionDoc {
        name: "MoveUnit",
        parameters: &["count", "unit", "player", "start_location", "dest_location"],
        typescript_parameters: &["count", "unit", "player", "srcLocation", "destLocation"],
        result: FunctionResult::Action,
        summary: "Teleports up to a count of units between locations.",
    },
    FunctionDoc {
        name: "GiveUnits",
        parameters: &["count", "unit", "player", "location", "new_player"],
        typescript_parameters: &["count", "unit", "player", "srcLocation", "toPlayer"],
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
        typescript_parameters: &["unit", "player", "srcLocation", "orderType", "destLocation"],
        result: FunctionResult::Action,
        summary: "Orders units at a location to move, patrol, or attack.",
    },
    FunctionDoc {
        name: "Switch",
        parameters: &["switch", "state"],
        typescript_parameters: &["switchId", "state"],
        result: FunctionResult::Condition,
        summary: "Checks a switch's state.",
    },
    FunctionDoc {
        name: "SetSwitch",
        parameters: &["switch", "state"],
        typescript_parameters: &["switchId", "action"],
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
            assert_eq!(
                doc.parameters.len(),
                doc.typescript_parameters.len(),
                "{} has different Python and TypeScript arity",
                function.name
            );
            for language in ["python", "typescript"] {
                let parameters = doc.parameters_for(language);
                for argument in function.arguments {
                    assert!(
                        parameters.get(argument.index).is_some(),
                        "{} has no {language} parameter at index {}",
                        function.name,
                        argument.index
                    );
                }
            }
        }
    }

    #[test]
    fn typescript_parameters_use_the_sdk_names() {
        let cases: &[(&str, &[&str], &[&str])] = &[
            ("Switch", &["switch", "state"], &["switchId", "state"]),
            ("SetSwitch", &["switch", "state"], &["switchId", "action"]),
            (
                "MoveLocation",
                &["location", "unit", "player", "dest_location"],
                &["location", "onUnit", "owner", "destLocation"],
            ),
            (
                "MoveUnit",
                &["count", "unit", "player", "start_location", "dest_location"],
                &["count", "unit", "player", "srcLocation", "destLocation"],
            ),
            (
                "GiveUnits",
                &["count", "unit", "player", "location", "new_player"],
                &["count", "unit", "player", "srcLocation", "toPlayer"],
            ),
            (
                "Order",
                &[
                    "unit",
                    "player",
                    "start_location",
                    "order_type",
                    "dest_location",
                ],
                &["unit", "player", "srcLocation", "orderType", "destLocation"],
            ),
        ];
        for &(name, python, typescript) in cases {
            let doc = function_doc(name).unwrap();
            assert_eq!(doc.parameters_for("python"), python, "{name}");
            assert_eq!(doc.parameters_for("typescript"), typescript, "{name}");
            assert_eq!(doc.parameters_for("javascript"), typescript, "{name}");
        }

        let text = function_doc("SetSwitch")
            .unwrap()
            .hover_markdown("typescript");
        assert!(text.contains("**SetSwitch(switchId, action)**"), "{text}");

        let text = function_doc("MoveLocation")
            .unwrap()
            .hover_markdown("javascript");
        assert!(
            text.contains("**MoveLocation(location, onUnit, owner, destLocation)**"),
            "{text}"
        );
        assert!(
            text.contains("`onUnit` accepts a canonical unit name"),
            "{text}"
        );
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
