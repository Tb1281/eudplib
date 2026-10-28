//! TypeScript and JavaScript analysis built on the oxc parser.
//!
//! Only the `KillUnit` unit-name slot is implemented, mirroring the Python
//! frontend. The parser's token stream is used instead of the AST because it is
//! available for complete documents and comments are not part of the token
//! sequence. While the user is typing an unterminated string, oxc reports a
//! fatal error and does not retain the literal, so a narrow fallback locates
//! the in-progress literal from the parser diagnostic.

use oxc_allocator::Allocator;
use oxc_parser::config::TokensParserConfig;
use oxc_parser::{Kind, Parser, Token};
use oxc_span::SourceType;

use crate::analysis::symbol_slot::UNIT_FUNCTION;

/// A TypeScript or JavaScript source language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsLanguage {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
}

impl TsLanguage {
    /// Map a VS Code language id to a TypeScript/JavaScript language.
    pub fn from_language_id(language_id: &str) -> Option<Self> {
        match language_id {
            "typescript" | "ts" => Some(Self::TypeScript),
            "typescriptreact" | "tsx" => Some(Self::Tsx),
            "javascript" | "js" => Some(Self::JavaScript),
            "javascriptreact" | "jsx" => Some(Self::Jsx),
            _ => None,
        }
    }

    fn source_type(self) -> SourceType {
        match self {
            Self::TypeScript => SourceType::ts(),
            Self::Tsx => SourceType::tsx(),
            Self::JavaScript => SourceType::unambiguous(),
            Self::Jsx => SourceType::jsx(),
        }
    }
}

/// The unit argument of a `KillUnit` call in a TypeScript or JavaScript file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsUnitNameSlot {
    content_start: usize,
    content_end: usize,
}

impl TsUnitNameSlot {
    /// Byte range of the existing literal content to replace on insertion.
    pub fn content_range(&self) -> std::ops::Range<usize> {
        self.content_start..self.content_end
    }
}

/// Find the `KillUnit` unit-name slot containing `offset`, if any.
pub fn ts_symbol_slot_at(
    source: &str,
    language: TsLanguage,
    offset: usize,
) -> Option<TsUnitNameSlot> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, language.source_type())
        .with_config(TokensParserConfig)
        .parse();

    if parsed.fatal_error {
        return unterminated_string_slot(source, &parsed.diagnostics, offset);
    }

    symbol_slot_from_tokens(source, &parsed.tokens, offset)
}

fn symbol_slot_from_tokens(
    source: &str,
    tokens: &[Token],
    offset: usize,
) -> Option<TsUnitNameSlot> {
    let (string_index, slot) = tokens.iter().enumerate().find_map(|(index, token)| {
        if token.kind() != Kind::Str {
            return None;
        }

        let span = token.span();
        let content_start = (span.start as usize).checked_add(1)?;
        let content_end = (span.end as usize).checked_sub(1)?;
        if offset < content_start || offset > content_end {
            return None;
        }

        Some((
            index,
            TsUnitNameSlot {
                content_start,
                content_end,
            },
        ))
    })?;

    is_kill_unit_first_argument(source, tokens, string_index).then_some(slot)
}

/// Whether `string_index` is the direct first argument of a `KillUnit` call.
fn is_kill_unit_first_argument(source: &str, tokens: &[Token], string_index: usize) -> bool {
    let mut openers: Vec<(Kind, usize)> = Vec::new();

    for (index, token) in tokens[..string_index].iter().enumerate() {
        match token.kind() {
            Kind::LParen | Kind::LBrack | Kind::LCurly => openers.push((token.kind(), index)),
            Kind::RParen => pop_if_matches(&mut openers, Kind::LParen),
            Kind::RBrack => pop_if_matches(&mut openers, Kind::LBrack),
            Kind::RCurly => pop_if_matches(&mut openers, Kind::LCurly),
            _ => {}
        }
    }

    let Some((kind, open_index)) = openers.last().copied() else {
        return false;
    };
    if kind != Kind::LParen || open_index == 0 {
        return false;
    }

    // The first argument must be the string literal itself; `KillUnit(foo("..."))`
    // and later arguments must not complete.
    if string_index != open_index + 1 {
        return false;
    }

    let name_token = &tokens[open_index - 1];
    if name_token.kind() != Kind::Ident {
        return false;
    }

    let name_span = name_token.span();
    if &source[name_span.start as usize..name_span.end as usize] != UNIT_FUNCTION {
        return false;
    }

    // Reject member calls and declarations such as `eud.KillUnit(...)` or
    // `function KillUnit(...)`.
    if open_index >= 2
        && matches!(
            tokens[open_index - 2].kind(),
            Kind::Dot | Kind::QuestionDot | Kind::Function | Kind::New | Kind::Class
        )
    {
        return false;
    }

    true
}

fn pop_if_matches(openers: &mut Vec<(Kind, usize)>, expected: Kind) {
    if openers.last().is_some_and(|(kind, _)| *kind == expected) {
        openers.pop();
    }
}

/// Recover an in-progress `KillUnit("` from oxc's fatal unterminated-string error.
///
/// The parser reports the opening quote and the extent of the unterminated
/// literal. Scanning backwards from that quote only accepts a literal that is
/// directly after a `KillUnit(` call opener, so comments, unrelated strings, and
/// later arguments cannot match.
fn unterminated_string_slot(
    source: &str,
    diagnostics: &oxc_diagnostics::Diagnostics,
    offset: usize,
) -> Option<TsUnitNameSlot> {
    diagnostics.iter().find_map(|diagnostic| {
        if diagnostic.message != "Unterminated string" {
            return None;
        }

        diagnostic.labels.iter().find_map(|label| {
            let span = label.span();
            let start = span.start as usize;
            let end = span.end as usize;

            if offset <= start || offset > end {
                return None;
            }

            let quote = source.get(start..start.checked_add(1)?)?;
            if quote != "\"" && quote != "'" {
                return None;
            }
            if !starts_kill_unit_call(source, start) {
                return None;
            }

            Some(TsUnitNameSlot {
                content_start: start + 1,
                content_end: end,
            })
        })
    })
}

/// Whether the string starting at `literal_start` directly follows `KillUnit(`.
fn starts_kill_unit_call(source: &str, literal_start: usize) -> bool {
    let Some(paren_end) = trim_trivia_end(source, literal_start) else {
        return false;
    };
    if !source[..paren_end].ends_with('(') {
        return false;
    }

    let Some(name_end) = trim_trivia_end(source, paren_end - 1) else {
        return false;
    };
    let name_start = identifier_start(source, name_end);
    if name_start == name_end || &source[name_start..name_end] != UNIT_FUNCTION {
        return false;
    }

    let Some(before_name_end) = trim_trivia_end(source, name_start) else {
        return true;
    };
    if before_name_end > 0 {
        let before_name = source[..before_name_end]
            .chars()
            .next_back()
            .expect("non-empty prefix has a character");
        if before_name == '.' {
            return false;
        }
    }

    let previous_start = identifier_start(source, before_name_end);
    !matches!(
        &source[previous_start..before_name_end],
        "function" | "new" | "class"
    )
}

/// End offset after trimming trailing whitespace and block comments.
fn trim_trivia_end(source: &str, mut end: usize) -> Option<usize> {
    loop {
        while end > 0 {
            let character = source[..end].chars().next_back()?;
            if character.is_whitespace() {
                end -= character.len_utf8();
            } else {
                break;
            }
        }

        if end >= 2 && source[..end].ends_with("*/") {
            end = source[..end - 2].rfind("/*")?;
            continue;
        }

        return Some(end);
    }
}

/// Start offset of the identifier ending at `end`.
fn identifier_start(source: &str, mut end: usize) -> usize {
    while end > 0 {
        let Some(character) = source[..end].chars().next_back() else {
            break;
        };
        if character.is_alphanumeric() || character == '_' || character == '$' {
            end -= character.len_utf8();
        } else {
            break;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    fn slot(source: &str, language: TsLanguage, offset: usize) -> Option<Range<usize>> {
        ts_symbol_slot_at(source, language, offset).map(|slot| slot.content_range())
    }

    #[test]
    fn completes_in_typescript_tsx_javascript_and_jsx() {
        for (language, source) in [
            (TsLanguage::TypeScript, "KillUnit(\""),
            (TsLanguage::Tsx, "const view = <div />;\nKillUnit(\""),
            (TsLanguage::JavaScript, "KillUnit(\""),
            (TsLanguage::Jsx, "const view = <div />;\nKillUnit(\""),
        ] {
            assert_eq!(
                slot(source, language, source.len()),
                Some(source.len()..source.len()),
                "{language:?} should complete the unterminated unit string"
            );
        }
    }

    #[test]
    fn accepts_single_quotes_partial_values_and_closed_strings() {
        for (language, source, offset) in [
            (
                TsLanguage::TypeScript,
                "KillUnit('Mar",
                "KillUnit('Mar".len(),
            ),
            (
                TsLanguage::JavaScript,
                "KillUnit('Mar",
                "KillUnit('Mar".len(),
            ),
            (
                TsLanguage::TypeScript,
                "KillUnit(\"Mar",
                "KillUnit(\"Mar".len(),
            ),
            (
                TsLanguage::JavaScript,
                "KillUnit(\"Terran Marine\", 1)",
                "KillUnit(\"Terr".len(),
            ),
            (
                TsLanguage::Tsx,
                "const view = <div />;\nKillUnit('Zerg Zergling', 0)",
                "const view = <div />;\nKillUnit('Zerg".len(),
            ),
        ] {
            assert!(
                slot(source, language, offset).is_some(),
                "{language:?} should complete {source:?}"
            );
        }
    }

    #[test]
    fn replaces_only_the_literal_content() {
        let source = "KillUnit(\"Terran Marine\", 1)";
        assert_eq!(
            slot(source, TsLanguage::TypeScript, "KillUnit(\"Terr".len()),
            Some(10..23)
        );

        let unterminated = "KillUnit(\"Mar";
        assert_eq!(
            slot(unterminated, TsLanguage::TypeScript, unterminated.len()),
            Some(10..unterminated.len())
        );
    }

    #[test]
    fn does_not_complete_outside_the_unit_argument() {
        let cases = [
            ("DisplayText(\"", "DisplayText(\"".len()),
            (
                "KillUnit(\"Terran Marine\", \"",
                "KillUnit(\"Terran Marine\", \"".len(),
            ),
            ("KillUnit(1, \"", "KillUnit(1, \"".len()),
            (
                "KillUnit(\"Terran Marine\", 1, \"",
                "KillUnit(\"Terran Marine\", 1, \"".len(),
            ),
            ("// KillUnit(\"\nconst x = 1;", "// KillUnit(\"".len()),
            (
                "const message = \"KillUnit(\";",
                "const message = \"KillUnit(\"".len(),
            ),
            ("const unit = \"Mar", "const unit = \"Mar".len()),
            ("eud.KillUnit(\"Mar\", 1)", "eud.KillUnit(\"Mar".len()),
            ("function KillUnit(\"Mar", "function KillUnit(\"Mar".len()),
        ];

        for (source, offset) in cases {
            assert_eq!(
                slot(source, TsLanguage::TypeScript, offset),
                None,
                "must not complete in {source:?}"
            );
        }
    }

    #[test]
    fn skips_comments_before_the_first_argument() {
        let source = "KillUnit(/* unit */ \"Mar\")";
        assert_eq!(
            slot(
                source,
                TsLanguage::TypeScript,
                "KillUnit(/* unit */ \"Mar".len()
            ),
            Some(21..24)
        );
    }

    #[test]
    fn maps_vscode_language_ids() {
        assert_eq!(
            TsLanguage::from_language_id("typescript"),
            Some(TsLanguage::TypeScript)
        );
        assert_eq!(
            TsLanguage::from_language_id("typescriptreact"),
            Some(TsLanguage::Tsx)
        );
        assert_eq!(
            TsLanguage::from_language_id("javascript"),
            Some(TsLanguage::JavaScript)
        );
        assert_eq!(
            TsLanguage::from_language_id("javascriptreact"),
            Some(TsLanguage::Jsx)
        );
        assert_eq!(TsLanguage::from_language_id("python"), None);
    }
}
