//! Locating the call under the cursor for signature help.

use std::ops::Range;

use oxc_allocator::Allocator;
use oxc_parser::config::TokensParserConfig;
use oxc_parser::{Kind, Parser, Token as OxcToken};
use ruff_text_size::Ranged;

use crate::analysis::call_site::enclosing_call;
use crate::analysis::document::ParsedDocument;
use crate::analysis::ts::{call_name, TsLanguage};

/// The call surrounding a cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    pub name: String,
    /// Zero-based positional argument index.
    pub active_parameter: usize,
    /// Keyword used for the active argument, if any.
    pub keyword: Option<String>,
}

/// Find the Python call surrounding `offset`.
pub fn python_call_context(source: &str, offset: usize) -> Option<CallContext> {
    let document = ParsedDocument::parse(source);
    let tokens = document.tokens();
    let token_index = tokens
        .iter()
        .position(|token| token.end().to_usize() > offset)
        .unwrap_or(tokens.len());
    let call = enclosing_call(tokens, source, token_index)?;

    Some(CallContext {
        name: call.name,
        active_parameter: call.argument_index,
        keyword: call.keyword,
    })
}

/// Find the TypeScript or JavaScript call surrounding `offset`.
pub fn ts_call_context(source: &str, language: TsLanguage, offset: usize) -> Option<CallContext> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, language.source_type())
        .with_config(TokensParserConfig)
        .parse();
    if parsed.fatal_error {
        return fallback_ts_call_context(source, offset);
    }

    let tokens = &parsed.tokens;
    let token_index = tokens
        .iter()
        .position(|token| token.span().end as usize > offset)
        .unwrap_or(tokens.len());
    let open_index = enclosing_paren(tokens, token_index)?;
    let name = call_name(source, tokens, open_index)?;
    let (active_parameter, keyword) = argument_state(source, tokens, open_index, token_index);

    Some(CallContext {
        name: name.to_string(),
        active_parameter,
        keyword,
    })
}

/// Source-only fallback used while oxc has no tokens for an unfinished call.
///
/// Oxc discards the token stream when a call is still open at EOF (for example
/// `KillUnit("`), which is exactly when signature help is most useful. This
/// scanner ignores strings and comments and tracks the innermost open call.
fn fallback_ts_call_context(source: &str, offset: usize) -> Option<CallContext> {
    #[derive(Debug)]
    struct Frame {
        open: usize,
        argument: usize,
        keyword: Option<String>,
        segment_start: usize,
        nested: usize,
    }

    let bytes = source.as_bytes();
    let mut frames: Vec<Frame> = Vec::new();
    let mut index = 0usize;

    while index < offset.min(bytes.len()) {
        match bytes[index] {
            b'\'' | b'"' | b'`' => {
                index = skip_string(bytes, index)?;
                continue;
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = skip_line_comment(bytes, index + 2);
                continue;
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = skip_block_comment(bytes, index + 2);
                continue;
            }
            b'(' => {
                frames.push(Frame {
                    open: index,
                    argument: 0,
                    keyword: None,
                    segment_start: index + 1,
                    nested: 0,
                });
            }
            b')' => {
                frames.pop();
            }
            b'[' | b'{' => {
                if let Some(frame) = frames.last_mut() {
                    frame.nested += 1;
                }
            }
            b']' | b'}' => {
                if let Some(frame) = frames.last_mut() {
                    frame.nested = frame.nested.saturating_sub(1);
                }
            }
            b',' => {
                if let Some(frame) = frames.last_mut() {
                    if frame.nested == 0 {
                        frame.argument += 1;
                        frame.keyword = None;
                        frame.segment_start = index + 1;
                    }
                }
            }
            b'=' => {
                if let Some(frame) = frames.last_mut() {
                    if frame.nested == 0 && frame.keyword.is_none() {
                        let segment = source[frame.segment_start..index].trim();
                        if !segment.is_empty()
                            && segment
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                        {
                            frame.keyword = Some(segment.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
        index += 1;
    }

    let frame = frames.last()?;
    let name_start = identifier_start_before(source, frame.open)?;
    let name = source.get(name_start..frame.open)?.trim();
    if name.is_empty() {
        return None;
    }

    Some(CallContext {
        name: name.to_string(),
        active_parameter: frame.argument,
        keyword: frame.keyword.clone(),
    })
}

fn skip_string(bytes: &[u8], start: usize) -> Option<usize> {
    let quote = *bytes.get(start)?;
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            byte if byte == quote => return Some(index + 1),
            _ => index += 1,
        }
    }
    Some(bytes.len())
}

fn skip_line_comment(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |newline| start + newline + 1)
}

fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .windows(2)
        .position(|window| window == b"*/")
        .map_or(bytes.len(), |end| start + end + 2)
}

fn identifier_start_before(source: &str, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = end;
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    let end = index;
    while index > 0 && is_identifier_byte(bytes[index - 1]) {
        index -= 1;
    }
    (index < end).then_some(index)
}

/// Index of the innermost open `(` before `token_index`.
fn enclosing_paren(tokens: &[OxcToken], token_index: usize) -> Option<usize> {
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

/// Zero-based argument index and keyword at the cursor.
fn argument_state(
    source: &str,
    tokens: &[OxcToken],
    open_index: usize,
    token_index: usize,
) -> (usize, Option<String>) {
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut keyword = None;
    let mut segment_start = open_index + 1;

    for position in open_index + 1..token_index {
        let kind = tokens[position].kind();
        match kind {
            Kind::LParen | Kind::LBrack | Kind::LCurly => depth += 1,
            Kind::RParen | Kind::RBrack | Kind::RCurly => {
                depth = depth.saturating_sub(1);
            }
            Kind::Comma if depth == 0 => {
                index += 1;
                keyword = None;
                segment_start = position + 1;
            }
            Kind::Eq
                if depth == 0
                    && keyword.is_none()
                    && position == segment_start + 1
                    && tokens[segment_start].kind() == Kind::Ident =>
            {
                let span = tokens[segment_start].span();
                keyword = source
                    .get(span.start as usize..span.end as usize)
                    .map(str::to_string);
            }
            _ => {}
        }
    }

    (index, keyword)
}

fn pop_if_matches(openers: &mut Vec<(Kind, usize)>, expected: Kind) -> bool {
    openers.last().is_some_and(|(kind, _)| *kind == expected) && {
        openers.pop();
        true
    }
}

/// Byte range of the identifier under or immediately before `offset`.
pub fn identifier_at(source: &str, offset: usize) -> Option<Range<usize>> {
    let bytes = source.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut position = offset.min(bytes.len());
    if position == bytes.len() || !is_identifier_byte(bytes[position]) {
        position = position.checked_sub(1)?;
    }
    if !is_identifier_byte(bytes[position]) {
        return None;
    }

    let mut start = position;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = position + 1;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }

    Some(start..end)
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

#[cfg(test)]
mod tests {
    use super::{identifier_at, python_call_context, ts_call_context};
    use crate::analysis::TsLanguage;

    #[test]
    fn finds_python_call_and_active_parameter() {
        let source = "KillUnit(";
        let context = python_call_context(source, source.len()).unwrap();
        assert_eq!(context.name, "KillUnit");
        assert_eq!(context.active_parameter, 0);
        assert_eq!(context.keyword, None);

        let source = "KillUnit(\"Terran Marine\", ";
        let context = python_call_context(source, source.len()).unwrap();
        assert_eq!(context.active_parameter, 1);

        let source = "KillUnit(player=1, unit=\"";
        let context = python_call_context(source, source.len()).unwrap();
        assert_eq!(context.active_parameter, 1);
        assert_eq!(context.keyword.as_deref(), Some("unit"));
    }

    #[test]
    fn finds_qualified_typescript_calls() {
        let source = "eudkit.KillUnit(";
        let context = ts_call_context(source, TsLanguage::TypeScript, source.len()).unwrap();
        assert_eq!(context.name, "KillUnit");
        assert_eq!(context.active_parameter, 0);

        let source = "KillUnit(\"Terran Marine\", ";
        let context = ts_call_context(source, TsLanguage::TypeScript, source.len()).unwrap();
        assert_eq!(context.active_parameter, 1);
    }

    #[test]
    fn ignores_non_calls_and_finds_identifiers() {
        assert!(python_call_context("value = 1", 5).is_none());
        assert!(ts_call_context("const value = 1;", TsLanguage::TypeScript, 5).is_none());

        assert_eq!(identifier_at("eudkit.KillUnit(\"", 14), Some(7..15));
        assert_eq!(identifier_at("KillUnit", 8), Some(0..8));
        assert_eq!(identifier_at("", 0), None);
    }
}
