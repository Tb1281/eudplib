//! TypeScript and JavaScript analysis built on the oxc parser.
//!
//! The shared symbol table decides which call arguments accept Unit, Location,
//! and Switch names. The parser's token stream is used instead of the AST
//! because it is available for complete documents and comments are not part of
//! the token sequence. While the user is typing an unterminated string, oxc
//! reports a fatal error and does not retain any tokens, so a narrow fallback
//! locates the in-progress literal from the parser diagnostic and scans the
//! preceding call arguments.

use oxc_allocator::Allocator;
use oxc_parser::config::TokensParserConfig;
use oxc_parser::{Kind, Parser, Token};
use oxc_span::SourceType;

use crate::analysis::symbol_slot::{function_symbols, SymbolKind};

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

/// A symbol-typed string argument in a TypeScript or JavaScript file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsSymbolSlot {
    kind: SymbolKind,
    content_start: usize,
    content_end: usize,
}

impl TsSymbolSlot {
    /// Byte range of the existing literal content to replace on insertion.
    pub fn content_range(&self) -> std::ops::Range<usize> {
        self.content_start..self.content_end
    }

    /// Kind of symbol accepted by this argument.
    pub fn kind(&self) -> SymbolKind {
        self.kind
    }
}

/// Find the symbol slot containing `offset`, if any.
pub fn ts_symbol_slot_at(
    source: &str,
    language: TsLanguage,
    offset: usize,
) -> Option<TsSymbolSlot> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, language.source_type())
        .with_config(TokensParserConfig)
        .parse();

    if parsed.fatal_error {
        return unterminated_string_slot(source, &parsed.diagnostics, offset);
    }

    symbol_slot_from_tokens(source, &parsed.tokens, offset)
}

fn symbol_slot_from_tokens(source: &str, tokens: &[Token], offset: usize) -> Option<TsSymbolSlot> {
    let (string_index, content_start, content_end) =
        tokens.iter().enumerate().find_map(|(index, token)| {
            if token.kind() != Kind::Str {
                return None;
            }

            let span = token.span();
            let content_start = (span.start as usize).checked_add(1)?;
            let content_end = (span.end as usize).checked_sub(1)?;
            if offset < content_start || offset > content_end {
                return None;
            }

            Some((index, content_start, content_end))
        })?;

    let kind = symbol_kind_for_string(source, tokens, string_index)?;
    Some(TsSymbolSlot {
        kind,
        content_start,
        content_end,
    })
}

/// Resolve the symbol kind of the string token at `string_index`.
fn symbol_kind_for_string(
    source: &str,
    tokens: &[Token],
    string_index: usize,
) -> Option<SymbolKind> {
    let open_index = enclosing_paren(tokens, string_index)?;
    let argument_index = argument_index(tokens, open_index, string_index)?;
    let function = function_symbols(call_name(source, tokens, open_index)?)?;
    let argument = function
        .arguments
        .iter()
        .find(|argument| argument.index == argument_index)?;

    Some(argument.kind)
}

/// Index of the innermost open `(` containing `token_index`.
fn enclosing_paren(tokens: &[Token], token_index: usize) -> Option<usize> {
    let mut openers: Vec<(Kind, usize)> = Vec::new();

    for (index, token) in tokens[..token_index].iter().enumerate() {
        match token.kind() {
            Kind::LParen | Kind::LBrack | Kind::LCurly => openers.push((token.kind(), index)),
            Kind::RParen if !pop_if_matches(&mut openers, Kind::LParen) => return None,
            Kind::RBrack if !pop_if_matches(&mut openers, Kind::LBrack) => return None,
            Kind::RCurly if !pop_if_matches(&mut openers, Kind::LCurly) => return None,
            _ => {}
        }
    }

    let (kind, open_index) = openers.last().copied()?;
    (kind == Kind::LParen && open_index > 0).then_some(open_index)
}

/// Zero-based argument index of a direct string argument, if it is direct.
fn argument_index(tokens: &[Token], open_index: usize, string_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = 0usize;

    for token in &tokens[open_index + 1..string_index] {
        match token.kind() {
            Kind::LParen | Kind::LBrack | Kind::LCurly => depth += 1,
            Kind::RParen | Kind::RBrack | Kind::RCurly => depth = depth.checked_sub(1)?,
            Kind::Comma if depth == 0 => index += 1,
            _ => {}
        }
    }

    if depth != 0 {
        return None;
    }

    let previous = tokens.get(string_index.checked_sub(1)?)?.kind();
    matches!(previous, Kind::LParen | Kind::Comma).then_some(index)
}

/// Function name immediately before an open parenthesis.
fn call_name<'a>(source: &'a str, tokens: &[Token], open_index: usize) -> Option<&'a str> {
    let name_token = tokens.get(open_index.checked_sub(1)?)?;
    if name_token.kind() != Kind::Ident {
        return None;
    }

    if open_index >= 2
        && matches!(
            tokens[open_index - 2].kind(),
            Kind::Dot | Kind::QuestionDot | Kind::Function | Kind::New | Kind::Class
        )
    {
        return None;
    }

    let span = name_token.span();
    source.get(span.start as usize..span.end as usize)
}

fn pop_if_matches(openers: &mut Vec<(Kind, usize)>, expected: Kind) -> bool {
    if openers.last().is_some_and(|(kind, _)| *kind == expected) {
        openers.pop();
        true
    } else {
        false
    }
}

/// Recover an in-progress string argument from oxc's fatal unterminated-string
/// error.
///
/// The parser reports the opening quote and the extent of the unterminated
/// literal but returns no tokens. A small forward scan over the preceding
/// source finds the innermost call, its argument index, and whether the
/// literal is a direct argument. Comments and complete strings are skipped, so
/// the fallback cannot match unrelated text.
fn unterminated_string_slot(
    source: &str,
    diagnostics: &oxc_diagnostics::Diagnostics,
    offset: usize,
) -> Option<TsSymbolSlot> {
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

            let (open_index, argument_index) = enclosing_call_before(source, start)?;
            let function = function_symbols(call_name_at(source, open_index)?)?;
            let argument = function
                .arguments
                .iter()
                .find(|argument| argument.index == argument_index)?;

            Some(TsSymbolSlot {
                kind: argument.kind,
                content_start: start + 1,
                content_end: end,
            })
        })
    })
}

#[derive(Debug, Clone, Copy)]
struct OpenDelimiter {
    kind: char,
    open: usize,
    commas: usize,
}

/// Find the innermost open call and argument index before `end`.
///
/// Only `(`, `[`, `{`, complete strings, and comments are significant. The
/// literal at `end` must be directly preceded by the call's `(` or by a
/// top-level comma.
fn enclosing_call_before(source: &str, end: usize) -> Option<(usize, usize)> {
    let mut stack: Vec<OpenDelimiter> = Vec::new();
    let mut last_significant = None;
    let mut index = 0;

    while index < end {
        let rest = &source[index..end];
        let character = rest.chars().next()?;

        match character {
            '/' if rest.starts_with("//") => {
                index += rest.find('\n').unwrap_or(rest.len());
            }
            '/' if rest.starts_with("/*") => {
                index += rest[2..].find("*/").map_or(rest.len(), |end| end + 4);
            }
            '"' | '\'' | '`' => {
                index = skip_string(source, index, character, end)?;
                last_significant = Some(character);
            }
            '(' | '[' | '{' => {
                stack.push(OpenDelimiter {
                    kind: character,
                    open: index,
                    commas: 0,
                });
                last_significant = Some(character);
                index += character.len_utf8();
            }
            ')' | ']' | '}' => {
                let expected = match character {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    _ => unreachable!(),
                };
                if !stack.last().is_some_and(|open| open.kind == expected) {
                    return None;
                }
                stack.pop();
                last_significant = Some(character);
                index += character.len_utf8();
            }
            ',' => {
                if let Some(open) = stack.last_mut() {
                    if open.kind == '(' {
                        open.commas += 1;
                    }
                }
                last_significant = Some(character);
                index += character.len_utf8();
            }
            _ => {
                if !character.is_whitespace() {
                    last_significant = Some(character);
                }
                index += character.len_utf8();
            }
        }
    }

    if !matches!(last_significant, Some('(' | ',')) {
        return None;
    }

    let open = stack.last()?;
    (open.kind == '(').then_some((open.open, open.commas))
}

/// Skip a complete string literal starting at `start`.
fn skip_string(source: &str, start: usize, quote: char, limit: usize) -> Option<usize> {
    let mut index = start + quote.len_utf8();
    let mut escaped = false;

    while index < limit {
        let character = source[index..].chars().next()?;
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            return Some(index + character.len_utf8());
        }
        index += character.len_utf8();
    }

    None
}

/// Function name immediately before `open_index`, using source trivia rules.
fn call_name_at(source: &str, open_index: usize) -> Option<&str> {
    let name_end = trim_trivia_end(source, open_index)?;
    let name_start = identifier_start(source, name_end);
    if name_start == name_end {
        return None;
    }

    if is_member_or_declaration(source, name_start) {
        return None;
    }

    source.get(name_start..name_end)
}

fn is_member_or_declaration(source: &str, name_start: usize) -> bool {
    let Some(before_end) = trim_trivia_end(source, name_start) else {
        return false;
    };

    if before_end > 0
        && source[..before_end]
            .chars()
            .next_back()
            .is_some_and(|character| character == '.')
    {
        return true;
    }

    let previous_start = identifier_start(source, before_end);
    matches!(
        &source[previous_start..before_end],
        "function" | "new" | "class"
    )
}

/// End offset after trimming trailing whitespace and comments.
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

        let line_start = source[..end].rfind('\n').map_or(0, |newline| newline + 1);
        if let Some(comment) = line_comment_start(source, line_start, end) {
            end = comment;
            continue;
        }

        return Some(end);
    }
}

/// Start of a `//` comment in `source[line_start..end]`, ignoring strings.
fn line_comment_start(source: &str, line_start: usize, end: usize) -> Option<usize> {
    let line = &source[line_start..end];
    let mut quote = None;
    let mut escaped = false;

    for (offset, character) in line.char_indices() {
        if let Some(current) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == current {
                quote = None;
            }
            continue;
        }

        if matches!(character, '"' | '\'' | '`') {
            quote = Some(character);
        } else if character == '/' && line[offset..].starts_with("//") {
            return Some(line_start + offset);
        }
    }

    None
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

    fn kind(source: &str, language: TsLanguage, offset: usize) -> Option<SymbolKind> {
        ts_symbol_slot_at(source, language, offset).map(|slot| slot.kind)
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

        let location = "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Nowhere\")";
        assert_eq!(
            slot(
                location,
                TsLanguage::TypeScript,
                location.find("Nowhere").unwrap()
            ),
            Some(location.find("Nowhere").unwrap()..location.find("Nowhere").unwrap() + 7)
        );
    }

    #[test]
    fn maps_unit_location_and_switch_arguments() {
        for (source, expected) in [
            ("KillUnit(\"", SymbolKind::Unit),
            ("Deaths(0, 0, 0, \"", SymbolKind::Unit),
            ("DeathsX(0, 0, 0, \"", SymbolKind::Unit),
            ("SetDeaths(0, 0, 0, \"", SymbolKind::Unit),
            ("SetDeathsX(0, 0, 0, \"", SymbolKind::Unit),
            ("Bring(AllPlayers, AtLeast, 1, \"", SymbolKind::Unit),
            (
                "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"",
                SymbolKind::Location,
            ),
            ("CreateUnit(1, \"", SymbolKind::Unit),
            ("CreateUnit(1, \"Terran Marine\", \"", SymbolKind::Location),
            ("KillUnitAt(1, \"", SymbolKind::Unit),
            ("KillUnitAt(1, \"Terran Marine\", \"", SymbolKind::Location),
            ("RemoveUnitAt(1, \"", SymbolKind::Unit),
            (
                "RemoveUnitAt(1, \"Terran Marine\", \"",
                SymbolKind::Location,
            ),
            ("MinimapPing(\"", SymbolKind::Location),
            ("MoveLocation(\"", SymbolKind::Location),
            ("MoveLocation(\"Anywhere\", \"", SymbolKind::Unit),
            ("MoveLocation(\"Anywhere\", 0, 0, \"", SymbolKind::Location),
            ("MoveUnit(1, \"", SymbolKind::Unit),
            ("MoveUnit(1, \"Terran Marine\", 0, \"", SymbolKind::Location),
            (
                "MoveUnit(1, \"Terran Marine\", 0, \"Anywhere\", \"",
                SymbolKind::Location,
            ),
            ("GiveUnits(1, \"", SymbolKind::Unit),
            (
                "GiveUnits(1, \"Terran Marine\", 0, \"",
                SymbolKind::Location,
            ),
            ("Order(\"", SymbolKind::Unit),
            ("Order(\"Terran Marine\", 0, \"", SymbolKind::Location),
            (
                "Order(\"Terran Marine\", 0, \"Anywhere\", 0, \"",
                SymbolKind::Location,
            ),
            ("Switch(\"", SymbolKind::Switch),
            ("SetSwitch(\"", SymbolKind::Switch),
        ] {
            assert_eq!(
                kind(source, TsLanguage::TypeScript, source.len()),
                Some(expected),
                "{source:?}"
            );
        }
    }

    #[test]
    fn skips_nested_arguments_and_comments_before_a_symbol() {
        for (source, expected) in [
            (
                "Bring(AllPlayers, AtLeast, 1, foo(\"x\"), \"",
                SymbolKind::Location,
            ),
            (
                "Bring(AllPlayers, AtLeast, 1, /* unit */ \"",
                SymbolKind::Unit,
            ),
            (
                "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", /* location */ \"",
                SymbolKind::Location,
            ),
            ("KillUnit(// unit\n\"", SymbolKind::Unit),
            (
                "const url = \"http://x\"; Bring(AllPlayers, AtLeast, 1, \"",
                SymbolKind::Unit,
            ),
            ("KillUnit // unit\n(\"", SymbolKind::Unit),
        ] {
            assert_eq!(
                kind(source, TsLanguage::TypeScript, source.len()),
                Some(expected),
                "{source:?}"
            );
        }
    }

    #[test]
    fn does_not_complete_outside_a_symbol_argument() {
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
            (
                "Bring(AllPlayers, AtLeast, 1, 0, 0, \"",
                "Bring(AllPlayers, AtLeast, 1, 0, 0, \"".len(),
            ),
            (
                "Bring(AllPlayers, AtLeast, 1, (\"Terran Marine\"), 0)",
                "Bring(AllPlayers, AtLeast, 1, (\"Terran".len(),
            ),
            ("KillUnit(DisplayText(\"", "KillUnit(DisplayText(\"".len()),
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
