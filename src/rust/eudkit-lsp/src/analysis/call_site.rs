//! Locating the call and argument that enclose a token.

use ruff_python_ast::token::{Token, TokenKind};
use ruff_text_size::Ranged;

/// A function call surrounding a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    /// Function name as written before the opening parenthesis.
    pub name: String,
    /// Zero-based positional argument index.
    pub argument_index: usize,
    /// Keyword used for the argument, if any.
    pub keyword: Option<String>,
    /// Whether the token is the complete argument expression.
    pub argument_is_direct: bool,
}

/// Find the innermost call whose argument list contains `token_index`.
///
/// Only tokens before `token_index` are inspected, so this works for calls that
/// are still being typed and do not have a closing parenthesis yet.
pub fn enclosing_call(tokens: &[Token], source: &str, token_index: usize) -> Option<CallSite> {
    let token_index = token_index.min(tokens.len());
    let mut openers: Vec<(TokenKind, usize)> = Vec::new();

    for (index, token) in tokens[..token_index].iter().enumerate() {
        match token.kind() {
            TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => {
                openers.push((token.kind(), index));
            }
            TokenKind::Rpar => pop_if_matches(&mut openers, TokenKind::Lpar),
            TokenKind::Rsqb => pop_if_matches(&mut openers, TokenKind::Lsqb),
            TokenKind::Rbrace => pop_if_matches(&mut openers, TokenKind::Lbrace),
            _ => {}
        }
    }

    let (kind, open_index) = *openers.last()?;
    if kind != TokenKind::Lpar || open_index == 0 {
        return None;
    }

    let name_index = open_index - 1;
    let name_token = &tokens[name_index];
    if name_token.kind() != TokenKind::Identifier {
        return None;
    }

    // `def KillUnit(...)` and `class KillUnit(...)` are not calls.
    if name_index > 0
        && matches!(
            tokens[name_index - 1].kind(),
            TokenKind::Def | TokenKind::Class
        )
    {
        return None;
    }

    let name = source[name_token.range()].to_string();
    let mut depth = 0usize;
    let mut argument_index = 0usize;
    let mut keyword = None;
    let mut value_start = open_index + 1;

    for index in open_index + 1..token_index {
        let kind = tokens[index].kind();
        match kind {
            TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => depth += 1,
            TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Comma if depth == 0 => {
                argument_index += 1;
                keyword = None;
                value_start = index + 1;
            }
            TokenKind::Equal if depth == 0 && keyword.is_none() => {
                let mut significant = (value_start..index)
                    .map(|candidate| &tokens[candidate])
                    .filter(|token| !is_trivia(token.kind()));
                if let (Some(parameter), None) = (significant.next(), significant.next()) {
                    if parameter.kind() == TokenKind::Identifier {
                        keyword = Some(source[parameter.range()].to_string());
                        value_start = index + 1;
                    }
                }
            }
            _ => {}
        }
    }

    let argument_is_direct = tokens[value_start..token_index]
        .iter()
        .all(|token| is_trivia(token.kind()));

    Some(CallSite {
        name,
        argument_index,
        keyword,
        argument_is_direct,
    })
}

fn pop_if_matches(openers: &mut Vec<(TokenKind, usize)>, expected: TokenKind) {
    if openers.last().is_some_and(|(kind, _)| *kind == expected) {
        openers.pop();
    }
}

fn is_trivia(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comment
            | TokenKind::Newline
            | TokenKind::NonLogicalNewline
            | TokenKind::Indent
            | TokenKind::Dedent
    )
}
