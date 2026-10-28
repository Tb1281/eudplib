//! Document parsing and token-level helpers for the eudkit language server.
//!
//! The parser is intentionally error tolerant: a document is usually incomplete
//! while the user is typing, so tokens are kept even when the parser reports
//! syntax or lexical errors.

use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::StringFlags;
use ruff_python_parser::{parse_unchecked, Mode, ParseOptions};
use ruff_text_size::{Ranged, TextSize};

/// A parsed Python document backed by its source text and token stream.
#[derive(Debug)]
pub struct ParsedDocument<'a> {
    source: &'a str,
    tokens: Vec<Token>,
}

impl<'a> ParsedDocument<'a> {
    /// Parse `source` as a Python module without discarding tokens on errors.
    pub fn parse(source: &'a str) -> Self {
        let parsed = parse_unchecked(source, ParseOptions::from(Mode::Module))
            .try_into_module()
            .expect("module parsing always produces a module");
        Self {
            source,
            tokens: parsed.tokens().iter().copied().collect(),
        }
    }

    /// Source text used to build this document.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Lexed tokens, including trivia such as comments and newlines.
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Text covered by `token`.
    pub fn token_text(&self, token: &Token) -> &'a str {
        &self.source[token.range()]
    }

    /// Find the string literal containing `offset`.
    ///
    /// The offset is a byte offset into the source and must be on a character
    /// boundary. The end of an unclosed literal counts as inside the literal so
    /// that `KillUnit("` and `KillUnit("Mar` both work while the user is typing.
    pub fn string_literal_at(&self, offset: usize) -> Option<StringLiteral<'a>> {
        let offset = TextSize::new(u32::try_from(offset).ok()?);

        self.tokens
            .iter()
            .enumerate()
            .find_map(|(token_index, token)| {
                if token.kind() != TokenKind::String {
                    return None;
                }

                let flags = token.string_flags()?;
                let start = token.start().to_usize();
                let end = token.end().to_usize();
                let text = &self.source[start..end];
                let prefix_len = text.find(['\'', '"'])?;
                let quote_len = flags.quote_len().to_usize();
                let content_start = start + prefix_len + quote_len;
                let content_end = if flags.is_unclosed() {
                    end
                } else {
                    end.checked_sub(quote_len)?
                };

                if content_start > content_end {
                    return None;
                }
                if offset < TextSize::new(u32::try_from(content_start).ok()?)
                    || offset > TextSize::new(u32::try_from(content_end).ok()?)
                {
                    return None;
                }

                Some(StringLiteral {
                    token_index,
                    text,
                    content_start,
                    content_end,
                    unclosed: flags.is_unclosed(),
                })
            })
    }
}

/// A string literal that contains a cursor offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringLiteral<'a> {
    pub token_index: usize,
    pub text: &'a str,
    /// Byte offset of the first character after the opening quote.
    pub content_start: usize,
    /// Byte offset just after the literal content and before a closing quote.
    pub content_end: usize,
    pub unclosed: bool,
}
