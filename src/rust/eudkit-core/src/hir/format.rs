//! Print lowering: parses Frontend format templates into `FormatPart` tokens.
//!
//! `FormatPart` and `PrintTarget` are part of the HIR contract in the parent
//! module; this module owns the C-style template parser consumed by Frontends
//! and the Backend's CPString lowering.

use super::{FormatPart, HirExpr};

/// Parses a C-style format template string (e.g. `"Score: %d, Hex: 0x%08X\n"`)
/// into a list of static and dynamic `FormatPart` tokens.
pub fn parse_format_template(template: &str, args: &[HirExpr]) -> Result<Vec<FormatPart>, String> {
    let mut parts = Vec::new();
    let mut arg_idx = 0;
    let mut cur_text = String::new();
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            let mut next_i = i + 1;
            if bytes[next_i] == b'%' {
                cur_text.push('%');
                i += 2;
                continue;
            }
            let mut is_hex = false;
            let mut is_dec = false;
            if bytes[next_i] == b'd' || bytes[next_i] == b'i' || bytes[next_i] == b'u' {
                is_dec = true;
                next_i += 1;
            } else if bytes[next_i] == b'x' || bytes[next_i] == b'X' || bytes[next_i] == b'p' {
                is_hex = true;
                next_i += 1;
            } else if bytes[next_i..].starts_with(b"08X") || bytes[next_i..].starts_with(b"08x") {
                is_hex = true;
                next_i += 3;
            }

            if is_dec || is_hex {
                if !cur_text.is_empty() {
                    parts.push(FormatPart::Text(std::mem::take(&mut cur_text)));
                }
                let arg = args.get(arg_idx).ok_or_else(|| {
                    format!("not enough arguments for format template (missing arg {arg_idx})")
                })?;
                arg_idx += 1;
                if is_dec {
                    parts.push(FormatPart::Decimal(arg.clone()));
                } else {
                    parts.push(FormatPart::Hex(arg.clone()));
                }
                i = next_i;
                continue;
            }
        }
        cur_text.push(bytes[i] as char);
        i += 1;
    }
    if !cur_text.is_empty() {
        parts.push(FormatPart::Text(cur_text));
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::parse_format_template;
    use crate::hir::{FormatPart, HirExpr};

    fn text(value: &str) -> FormatPart {
        FormatPart::Text(value.to_string())
    }

    #[test]
    fn parses_mixed_specifiers_and_text() {
        let args = vec![
            HirExpr::Const(1),
            HirExpr::Const(0xABCD),
            HirExpr::Const(100),
        ];
        let parts = parse_format_template("Player %d: score=0x%08X, hp=%d", &args).unwrap();

        assert_eq!(
            parts,
            vec![
                text("Player "),
                FormatPart::Decimal(HirExpr::Const(1)),
                text(": score=0x"),
                FormatPart::Hex(HirExpr::Const(0xABCD)),
                text(", hp="),
                FormatPart::Decimal(HirExpr::Const(100)),
            ]
        );
    }

    #[test]
    fn parses_decimal_and_hex_specifier_variants() {
        let args = vec![
            HirExpr::Const(1),
            HirExpr::Const(2),
            HirExpr::Const(3),
            HirExpr::Const(4),
            HirExpr::Const(5),
        ];
        let parts = parse_format_template("%i|%u|%x|%X|%p", &args).unwrap();

        assert_eq!(
            parts,
            vec![
                FormatPart::Decimal(HirExpr::Const(1)),
                text("|"),
                FormatPart::Decimal(HirExpr::Const(2)),
                text("|"),
                FormatPart::Hex(HirExpr::Const(3)),
                text("|"),
                FormatPart::Hex(HirExpr::Const(4)),
                text("|"),
                FormatPart::Hex(HirExpr::Const(5)),
            ]
        );
    }

    #[test]
    fn treats_double_percent_as_literal_text() {
        let parts = parse_format_template("100%% ready", &[]).unwrap();

        assert_eq!(parts, vec![text("100% ready")]);
    }

    #[test]
    fn leaves_unsupported_specifiers_and_trailing_percent_literal() {
        let parts = parse_format_template("%s %", &[]).unwrap();

        assert_eq!(parts, vec![text("%s %")]);
    }

    #[test]
    fn errors_when_arguments_are_missing() {
        let err = parse_format_template("%d %d", &[HirExpr::Const(1)]).unwrap_err();

        assert_eq!(
            err,
            "not enough arguments for format template (missing arg 1)"
        );
    }

    #[test]
    fn ignores_extra_arguments_and_handles_empty_template() {
        assert_eq!(
            parse_format_template("", &[HirExpr::Const(1)]).unwrap(),
            Vec::new()
        );
        assert_eq!(
            parse_format_template("%d", &[HirExpr::Const(1), HirExpr::Const(2)]).unwrap(),
            vec![FormatPart::Decimal(HirExpr::Const(1))]
        );
    }
}
