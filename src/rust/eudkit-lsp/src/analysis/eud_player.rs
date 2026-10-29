//! Warnings for group players used with clear EUD unit offsets.
//!
//! `Deaths`, `DeathsX`, `SetDeaths`, and `SetDeathsX` reuse the unit field as
//! a memory offset when the player selector is a concrete player. With a group
//! player the same numeric unit value is ambiguous: it may be a normal unit ID
//! or an EUD offset. Only direct, unambiguous arguments are diagnosed.

use std::ops::Range;

use oxc_allocator::Allocator;
use oxc_parser::config::TokensParserConfig;
use oxc_parser::{Kind, Parser, Token as OxcToken};
use ruff_python_ast::token::{Token, TokenKind};
use ruff_text_size::Ranged;

use crate::analysis::document::ParsedDocument;
use crate::analysis::ts::{call_name, TsLanguage};
use eudkit_core::hir::PlayerId;
use eudkit_core::vocabulary::{VocabEntry, PLAYER_ENTRIES};

const EUD_FUNCTIONS: &[&str] = &["Deaths", "DeathsX", "SetDeaths", "SetDeathsX"];

const STOCK_UNIT_ID_MAX: i64 = 232;

/// An EUD player-selector warning at a byte range in the source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EudPlayerDiagnostic {
    pub range: Range<usize>,
    pub message: String,
}

/// EUD group-player warnings for a Python document.
pub fn python_eud_player_diagnostics(source: &str) -> Vec<EudPlayerDiagnostic> {
    let document = ParsedDocument::parse(source);
    python_calls(&document)
        .into_iter()
        .filter_map(|(function, arguments)| diagnostic_for_call(function, &arguments))
        .collect()
}

/// EUD group-player warnings for a TypeScript or JavaScript document.
pub fn ts_eud_player_diagnostics(source: &str, language: TsLanguage) -> Vec<EudPlayerDiagnostic> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, language.source_type())
        .with_config(TokensParserConfig)
        .parse();

    if parsed.fatal_error {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    for (index, token) in parsed.tokens.iter().enumerate() {
        if token.kind() != Kind::LParen {
            continue;
        }

        let Some(function) = call_name(source, &parsed.tokens, index) else {
            continue;
        };
        if !EUD_FUNCTIONS.contains(&function) {
            continue;
        }

        let arguments = ts_arguments(source, &parsed.tokens, index);
        diagnostics.extend(diagnostic_for_call(function, &arguments));
    }

    diagnostics
}

#[derive(Debug, Clone, Copy)]
struct GroupPlayer {
    name: Option<&'static str>,
    id: i64,
}

#[derive(Debug, Clone)]
struct CallArgument<'a> {
    keyword: Option<&'a str>,
    position: Option<usize>,
    range: Range<usize>,
    text: &'a str,
}

/// Warning text when `player` resolves to a group selector and `unit` resolves
/// to an EUD unit offset for one of the EUD trigger functions.
///
/// This is the pure, frontend-independent rule. Language scanners only locate
/// direct argument expressions and attach the returned message to the player
/// argument's source range.
pub fn eud_player_warning(function: &str, player: &str, unit: &str) -> Option<String> {
    if !EUD_FUNCTIONS.contains(&function) {
        return None;
    }

    let player = parse_group_player(player)?;
    let unit = parse_integer_literal(unit)?;

    (unit > STOCK_UNIT_ID_MAX).then(|| warning_message(function, player, unit))
}

fn diagnostic_for_call(
    function: &str,
    arguments: &[CallArgument<'_>],
) -> Option<EudPlayerDiagnostic> {
    let player_argument = argument(arguments, 0, "player")?;
    let unit_argument = argument(arguments, 3, "unit")?;
    let message = eud_player_warning(function, player_argument.text, unit_argument.text)?;

    Some(EudPlayerDiagnostic {
        range: player_argument.range.clone(),
        message,
    })
}

fn argument<'a>(
    arguments: &'a [CallArgument<'a>],
    index: usize,
    keyword: &str,
) -> Option<&'a CallArgument<'a>> {
    arguments
        .iter()
        .find(|argument| argument.keyword == Some(keyword))
        .or_else(|| {
            arguments
                .iter()
                .find(|argument| argument.keyword.is_none() && argument.position == Some(index))
        })
}

fn parse_group_player(expression: &str) -> Option<GroupPlayer> {
    let expression = strip_outer_parens(expression);

    if let Some(id) = parse_integer_literal(expression) {
        return group_player_by_id(id);
    }

    let name = identifier_path(expression)?;
    group_player_by_name(name)
}

fn group_players() -> impl Iterator<Item = &'static VocabEntry<PlayerId>> {
    PLAYER_ENTRIES.iter().filter(|entry| entry.value.is_group())
}

fn group_player_by_name(name: &str) -> Option<GroupPlayer> {
    group_players()
        .find(|entry| entry.name == name)
        .map(|entry| GroupPlayer {
            name: Some(entry.name),
            id: i64::from(entry.code),
        })
}

fn group_player_by_id(id: i64) -> Option<GroupPlayer> {
    group_players()
        .find(|entry| i64::from(entry.code) == id)
        .map(|entry| GroupPlayer {
            name: Some(entry.name),
            id: i64::from(entry.code),
        })
}

/// Last segment of a bare or member-qualified identifier expression.
fn identifier_path(expression: &str) -> Option<&str> {
    let mut last = None;
    for segment in expression.split('.') {
        let segment = segment.trim();
        if !is_identifier(segment) {
            return None;
        }
        last = Some(segment);
    }
    last
}

fn is_identifier(segment: &str) -> bool {
    let mut characters = segment.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }

    characters
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '$')
}

/// Parse an integer literal with optional wrapping parentheses and unary plus.
fn parse_integer_literal(expression: &str) -> Option<i64> {
    let expression = strip_outer_parens(expression);
    let expression = expression
        .strip_prefix('+')
        .map_or(expression, str::trim_start);

    let (radix, digits) = expression
        .strip_prefix("0x")
        .or_else(|| expression.strip_prefix("0X"))
        .map_or((10, expression), |digits| (16, digits));

    if digits.is_empty() {
        return None;
    }

    let digits: String = digits
        .chars()
        .filter(|character| *character != '_')
        .collect();
    if !digits.chars().all(|character| character.is_digit(radix)) {
        return None;
    }

    i64::from_str_radix(&digits, radix).ok()
}

/// Strip parentheses that wrap the entire expression.
fn strip_outer_parens(mut expression: &str) -> &str {
    loop {
        let trimmed = expression.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') || !wraps_entire_expression(trimmed)
        {
            return trimmed;
        }
        expression = &trimmed[1..trimmed.len() - 1];
    }
}

fn wraps_entire_expression(expression: &str) -> bool {
    let mut depth = 0usize;

    for (index, character) in expression.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
                if depth == 0 {
                    return index + 1 == expression.len();
                }
            }
            _ => {}
        }
    }

    false
}

fn warning_message(function: &str, player: GroupPlayer, unit: i64) -> String {
    let player = match player.name {
        Some(name) => format!("\"{name}\""),
        None => player.id.to_string(),
    };

    format!(
        "Group player {player} with unit offset {unit} is ambiguous in {function}: \
         the unit field is an EUD memory offset, not a unit type. Use a concrete \
         player such as Player1 for EUD triggers."
    )
}

fn python_calls<'a>(document: &ParsedDocument<'a>) -> Vec<(&'a str, Vec<CallArgument<'a>>)> {
    let source = document.source();
    let tokens = document.tokens();
    let mut calls = Vec::new();
    let mut index = 0usize;

    while index < tokens.len() {
        let token = &tokens[index];
        if token.kind() == TokenKind::Identifier {
            let function = &source[token.range()];
            if EUD_FUNCTIONS.contains(&function) && !is_definition(tokens, index) {
                if let Some(open) = next_significant(tokens, index + 1) {
                    if tokens[open].kind() == TokenKind::Lpar {
                        calls.push((function, python_arguments(source, tokens, open)));
                        index = open + 1;
                        continue;
                    }
                }
            }
        }
        index += 1;
    }

    calls
}

fn is_definition(tokens: &[Token], index: usize) -> bool {
    previous_significant(tokens, index).is_some_and(|previous| {
        matches!(tokens[previous].kind(), TokenKind::Def | TokenKind::Class)
    })
}

fn next_significant(tokens: &[Token], start: usize) -> Option<usize> {
    (start..tokens.len()).find(|index| !python_trivia(tokens[*index].kind()))
}

fn previous_significant(tokens: &[Token], end: usize) -> Option<usize> {
    (0..end)
        .rev()
        .find(|index| !python_trivia(tokens[*index].kind()))
}

fn python_arguments<'a>(source: &'a str, tokens: &[Token], open: usize) -> Vec<CallArgument<'a>> {
    let mut arguments = Vec::new();
    let mut position = 0usize;
    let mut depth = 0usize;
    let mut segment_start = open + 1;
    let mut index = open + 1;

    while index < tokens.len() {
        match tokens[index].kind() {
            TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => depth += 1,
            TokenKind::Rpar if depth == 0 => {
                push_python_argument(
                    source,
                    tokens,
                    segment_start..index,
                    &mut position,
                    &mut arguments,
                );
                return arguments;
            }
            TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Comma if depth == 0 => {
                push_python_argument(
                    source,
                    tokens,
                    segment_start..index,
                    &mut position,
                    &mut arguments,
                );
                segment_start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }

    push_python_argument(
        source,
        tokens,
        segment_start..tokens.len(),
        &mut position,
        &mut arguments,
    );
    arguments
}

fn push_python_argument<'a>(
    source: &'a str,
    tokens: &[Token],
    range: Range<usize>,
    position: &mut usize,
    arguments: &mut Vec<CallArgument<'a>>,
) {
    let significant: Vec<&Token> = tokens[range]
        .iter()
        .filter(|token| !python_trivia(token.kind()))
        .collect();
    let Some(first) = significant.first() else {
        return;
    };

    if matches!(first.kind(), TokenKind::Star | TokenKind::DoubleStar) {
        return;
    }

    let (keyword, value_tokens) = if significant.len() >= 2
        && first.kind() == TokenKind::Identifier
        && significant[1].kind() == TokenKind::Equal
    {
        (Some(&source[first.range()]), &significant[2..])
    } else {
        (None, &significant[..])
    };

    let (Some(value_first), Some(value_last)) = (value_tokens.first(), value_tokens.last()) else {
        return;
    };

    let range = value_first.start().to_usize()..value_last.end().to_usize();
    let argument_position = if keyword.is_none() {
        let current = *position;
        *position += 1;
        Some(current)
    } else {
        None
    };

    arguments.push(CallArgument {
        keyword,
        position: argument_position,
        text: &source[range.clone()],
        range,
    });
}

fn python_trivia(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comment
            | TokenKind::Newline
            | TokenKind::NonLogicalNewline
            | TokenKind::Indent
            | TokenKind::Dedent
    )
}

fn ts_arguments<'a>(source: &'a str, tokens: &[OxcToken], open: usize) -> Vec<CallArgument<'a>> {
    let mut arguments = Vec::new();
    let mut position = 0usize;
    let mut depth = 0usize;
    let mut segment_start = open + 1;
    let mut index = open + 1;

    while index < tokens.len() {
        match tokens[index].kind() {
            Kind::LParen | Kind::LBrack | Kind::LCurly => depth += 1,
            Kind::RParen if depth == 0 => {
                push_ts_argument(
                    source,
                    &tokens[segment_start..index],
                    &mut position,
                    &mut arguments,
                );
                return arguments;
            }
            Kind::RParen | Kind::RBrack | Kind::RCurly => {
                depth = depth.saturating_sub(1);
            }
            Kind::Comma if depth == 0 => {
                push_ts_argument(
                    source,
                    &tokens[segment_start..index],
                    &mut position,
                    &mut arguments,
                );
                segment_start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }

    push_ts_argument(
        source,
        &tokens[segment_start..],
        &mut position,
        &mut arguments,
    );
    arguments
}

fn push_ts_argument<'a>(
    source: &'a str,
    tokens: &[OxcToken],
    position: &mut usize,
    arguments: &mut Vec<CallArgument<'a>>,
) {
    let (Some(first), Some(last)) = (tokens.first(), tokens.last()) else {
        return;
    };

    if first.kind() == Kind::Dot3 {
        return;
    }

    let range = first.span().start as usize..last.span().end as usize;
    let argument_position = *position;
    *position += 1;

    arguments.push(CallArgument {
        keyword: None,
        position: Some(argument_position),
        text: &source[range.clone()],
        range,
    });
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::{eud_player_warning, python_eud_player_diagnostics, ts_eud_player_diagnostics};
    use crate::analysis::TsLanguage;

    #[test]
    fn pure_rule_requires_an_eud_function_group_player_and_offset() {
        let warning = eud_player_warning("Deaths", "Foes", "300").unwrap();
        assert!(warning.contains("ambiguous"));
        assert!(warning.contains("Player1"));

        assert!(eud_player_warning("Deaths", "Foes", "232").is_none());
        assert!(eud_player_warning("Deaths", "Player1", "300").is_none());
        assert!(eud_player_warning("Bring", "AllPlayers", "300").is_none());
    }

    fn python_ranges(source: &str) -> Vec<Range<usize>> {
        python_eud_player_diagnostics(source)
            .into_iter()
            .map(|diagnostic| diagnostic.range)
            .collect()
    }

    fn ts_ranges(source: &str, language: TsLanguage) -> Vec<Range<usize>> {
        ts_eud_player_diagnostics(source, language)
            .into_iter()
            .map(|diagnostic| diagnostic.range)
            .collect()
    }

    #[test]
    fn warns_when_a_group_player_uses_an_eud_offset_in_python() {
        let source = "Deaths(Foes, Exactly, 15, 300)";
        let diagnostics = python_eud_player_diagnostics(source);

        assert_eq!(diagnostics.len(), 1);
        let player_start = source.find("Foes").unwrap();
        assert_eq!(
            diagnostics[0].range,
            player_start..player_start + "Foes".len()
        );
        assert!(diagnostics[0].message.contains("ambiguous"));
        assert!(diagnostics[0].message.contains("Player1"));
    }

    #[test]
    fn warns_for_all_eud_functions_group_players_and_literal_forms() {
        let cases = [
            ("Deaths(Foes, Exactly, 15, 300)", "Foes"),
            ("Deaths(Force1, Exactly, 15, 300)", "Force1"),
            ("Deaths(AllPlayers, Exactly, 15, 300)", "AllPlayers"),
            ("DeathsX(AllPlayers, Exactly, 15, 300)", "AllPlayers"),
            ("SetDeaths(c.Foes, Exactly, 15, 0x12C)", "c.Foes"),
            (
                "SetDeathsX(eud.AllPlayers, Exactly, 15, (300))",
                "eud.AllPlayers",
            ),
            ("Deaths((Foes), Exactly, 15, +300)", "(Foes)"),
            ("Deaths(14, Exactly, 15, 300)", "14"),
            ("Deaths(0x11, Exactly, 15, 300)", "0x11"),
            ("Deaths(player=Foes, unit=300)", "Foes"),
            (
                "Deaths(player=Foes, comparison=Exactly, amount=15, unit=300)",
                "Foes",
            ),
            ("Deaths(Foes, Exactly, 15, unit=300)", "Foes"),
        ];

        for (source, player) in cases {
            let ranges = python_ranges(source);
            assert_eq!(ranges.len(), 1, "{source}");
            let start = source.find(player).unwrap();
            assert_eq!(ranges[0], start..start + player.len(), "{source}");
        }
    }

    #[test]
    fn warns_only_above_the_stock_unit_id_boundary() {
        assert!(python_ranges("Deaths(Foes, Exactly, 15, 232)").is_empty());
        assert_eq!(python_ranges("Deaths(Foes, Exactly, 15, 233)").len(), 1);
    }

    #[test]
    fn accepts_stock_players_stock_filters_variables_and_arithmetic() {
        let source = concat!(
            "Deaths(Foes, Exactly, 15, \"Any unit\")\n",
            "Deaths(Foes, Exactly, 15, 229)\n",
            "Deaths(Foes, Exactly, 15, 0)\n",
            "Deaths(Foes, Exactly, 15, 232)\n",
            "Deaths(Player1, Exactly, 15, 300)\n",
            "Deaths(CurrentPlayer, Exactly, 15, 300)\n",
            "Deaths(player, Exactly, 15, 300)\n",
            "Deaths(Foe, Exactly, 15, 300)\n",
            "Deaths(Foes, Exactly, 15, unit)\n",
            "Deaths(Foes + 1, Exactly, 15, 300)\n",
            "Deaths(Foes, Exactly, 15, 232 + 1)\n",
            "Deaths(Foes, Exactly, 15, 300.0)\n",
            "Deaths(Foes, Exactly, 15, -300)\n",
            "Deaths(Foes, Exactly, 15, \"300\")\n",
            "Deaths(getPlayer(), Exactly, 15, 300)\n",
            "Deaths(AllPlayers.id, Exactly, 15, 300)\n",
            "Deaths(Foes, Exactly, 15, getUnit(300))\n",
            "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Anywhere\")\n",
            "Bring(AllPlayers, Exactly, 15, 300, 0)\n",
        );

        assert!(
            python_ranges(source).is_empty(),
            "stock and ambiguous arguments must not warn"
        );
    }

    #[test]
    fn ignores_definitions_and_comments() {
        let source = concat!(
            "def Deaths(Foes, Exactly, 15, 300):\n",
            "    pass\n",
            "class Deaths(Foes, Exactly, 15, 300):\n",
            "    pass\n",
            "# Deaths(Foes, Exactly, 15, 300)\n",
            "message = \"Deaths(Foes, Exactly, 15, 300)\"\n",
        );

        assert!(python_ranges(source).is_empty());
    }

    #[test]
    fn warns_in_typescript_tsx_javascript_and_jsx() {
        let source = concat!(
            "Deaths(Foes, Exactly, 15, 300);\n",
            "DeathsX(AllPlayers, Exactly, 15, 0x12C);\n",
            "SetDeaths(c.Foes, Exactly, 15, (300));\n",
            "SetDeathsX(eud.AllPlayers, Exactly, 15, +300);\n",
        );

        for language in [
            TsLanguage::TypeScript,
            TsLanguage::Tsx,
            TsLanguage::JavaScript,
            TsLanguage::Jsx,
        ] {
            let diagnostics = ts_eud_player_diagnostics(source, language);
            assert_eq!(diagnostics.len(), 4, "{language:?}");

            for (diagnostic, player) in
                diagnostics
                    .iter()
                    .zip(["Foes", "AllPlayers", "c.Foes", "eud.AllPlayers"])
            {
                let start = source.find(player).unwrap();
                assert_eq!(
                    diagnostic.range,
                    start..start + player.len(),
                    "{language:?}"
                );
                assert!(diagnostic.message.contains("ambiguous"));
                assert!(diagnostic.message.contains("Player1"));
            }
        }
    }

    #[test]
    fn accepts_stock_and_ambiguous_arguments_in_typescript_and_javascript() {
        let source = concat!(
            "Deaths(Foes, Exactly, 15, \"Any unit\");\n",
            "Deaths(Foes, Exactly, 15, 229);\n",
            "Deaths(Foes, Exactly, 15, 0);\n",
            "Deaths(Foes, Exactly, 15, 232);\n",
            "Deaths(Player1, Exactly, 15, 300);\n",
            "Deaths(CurrentPlayer, Exactly, 15, 300);\n",
            "Deaths(player, Exactly, 15, 300);\n",
            "Deaths(Foe, Exactly, 15, 300);\n",
            "Deaths(Foes, Exactly, 15, unit);\n",
            "Deaths(Foes + 1, Exactly, 15, 300);\n",
            "Deaths(Foes, Exactly, 15, 232 + 1);\n",
            "Deaths(Foes, Exactly, 15, 300.0);\n",
            "Deaths(Foes, Exactly, 15, -300);\n",
            "Deaths(Foes, Exactly, 15, \"300\");\n",
            "Deaths(getPlayer(), Exactly, 15, 300);\n",
            "Deaths(AllPlayers.id, Exactly, 15, 300);\n",
            "Deaths(Foes, Exactly, 15, getUnit(300));\n",
            "Bring(AllPlayers, AtLeast, 1, \"Terran Marine\", \"Anywhere\");\n",
            "Bring(AllPlayers, Exactly, 15, 300, 0);\n",
        );

        for language in [
            TsLanguage::TypeScript,
            TsLanguage::Tsx,
            TsLanguage::JavaScript,
            TsLanguage::Jsx,
        ] {
            assert!(
                ts_ranges(source, language).is_empty(),
                "{language:?} must not warn about stock or ambiguous arguments"
            );
        }
    }

    #[test]
    fn ignores_typescript_declarations_and_comments() {
        let source = concat!(
            "function Deaths(Foes: number, a: number, b: number, c: number) {}\n",
            "class Deaths {}\n",
            "// Deaths(Foes, Exactly, 15, 300);\n",
            "const message = \"Deaths(Foes, Exactly, 15, 300)\";\n",
        );

        assert!(ts_ranges(source, TsLanguage::TypeScript).is_empty());
    }
}
