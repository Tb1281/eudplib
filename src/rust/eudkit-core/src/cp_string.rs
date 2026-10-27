use crate::hir::{epd, CURRENT_PLAYER_ADDR};
use crate::lir::{LirAction, ValueExpr};

/// Emits CPString streaming actions that write `dwords` to a destination buffer
/// relative to `dest_epd` by grouping writes into 12 residue classes `m = dword_index % 12`.
///
/// Within each residue class `m in 0..11`, writes are emitted as:
/// `SetDeaths(CurrentPlayer, SetTo, dword, unit=k)` where `k = dword_index / 12`
/// with ZERO touches to `0x6509B0`.
///
/// Between consecutive non-empty residue classes, `0x6509B0` is advanced by `Add delta`
/// at most 11 times total.
///
/// The total number of emitted actions is at most `N + 11`, requiring zero runtime
/// XOR decryption loops or runtime trigger branches.
pub fn emit_cpstring_residue_stream(
    dest_epd: ValueExpr,
    dwords: &[u32],
) -> Vec<LirAction> {
    let exprs: Vec<ValueExpr> = dwords
        .iter()
        .map(|&d| ValueExpr::Const(d as i32))
        .collect();
    emit_cpstring_residue_stream_exprs(dest_epd, &exprs)
}

/// Variant of `emit_cpstring_residue_stream` that accepts arbitrary `ValueExpr` dwords,
/// enabling dynamic variables, field references, and constant text chunks in the same stream.
pub fn emit_cpstring_residue_stream_exprs(
    _dest_epd: ValueExpr,
    dwords: &[ValueExpr],
) -> Vec<LirAction> {
    if dwords.is_empty() {
        return Vec::new();
    }

    // Group dword indices by residue class m = d % 12
    let mut residue_classes: [Vec<(u16, ValueExpr)>; 12] = Default::default();
    for (d, val) in dwords.iter().enumerate() {
        let m = d % 12;
        let k = (d / 12) as u16;
        residue_classes[m].push((k, *val));
    }

    // Find non-empty residue classes in ascending order
    let non_empty: Vec<usize> = (0..12).filter(|&m| !residue_classes[m].is_empty()).collect();
    if non_empty.is_empty() {
        return Vec::new();
    }

    let mut actions = Vec::with_capacity(dwords.len() + 12);
    let mut current_m = non_empty[0];

    for (idx, &m) in non_empty.iter().enumerate() {
        if idx > 0 {
            let delta = (m - current_m) as i32;
            actions.push(LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)),
                player2: ValueExpr::Const(delta),
                unitid: 0,
                acttype: 45, // SetDeaths
                amount: 8,  // Add
                flags: 20,
                eudx: 0,
            });
            current_m = m;
        }

        // Emit all writes for this residue class with unit=k
        for &(k, val) in &residue_classes[m] {
            actions.push(LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(13), // CurrentPlayer
                player2: val,
                unitid: k,
                acttype: 45, // SetDeaths
                amount: 7,  // SetTo
                flags: 20,
                eudx: 0,
            });
        }
    }

    actions
}

/// Emits the complete sequence of actions to write `dwords` to `dest_epd`:
/// 1. Initializes `0x6509B0` (`CurrentPlayer`) to `dest_epd + first_m`.
/// 2. Streams dwords using `emit_cpstring_residue_stream_exprs`.
/// 3. Restores `0x6509B0` to `restore_cp` (if provided, usually 0 for Player 1).
pub fn emit_cpstring_write_block(
    dest_epd: ValueExpr,
    dwords: &[ValueExpr],
    restore_cp: Option<i32>,
) -> Vec<LirAction> {
    if dwords.is_empty() {
        return Vec::new();
    }

    let mut actions = Vec::with_capacity(dwords.len() + 14);

    // Initial setup: CurrentPlayer = dest_epd
    actions.push(LirAction {
        locid1: ValueExpr::Const(0),
        strid: ValueExpr::Const(0),
        wavid: ValueExpr::Const(0),
        time: ValueExpr::Const(0),
        player1: ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)),
        player2: dest_epd,
        unitid: 0,
        acttype: 45, // SetDeaths
        amount: 7,  // SetTo
        flags: 20,
        eudx: 0,
    });

    let stream = emit_cpstring_residue_stream_exprs(dest_epd, dwords);
    actions.extend(stream);

    if let Some(cp) = restore_cp {
        actions.push(LirAction {
            locid1: ValueExpr::Const(0),
            strid: ValueExpr::Const(0),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)),
            player2: ValueExpr::Const(cp),
            unitid: 0,
            acttype: 45,
            amount: 7,
            flags: 20,
            eudx: 0,
        });
    }

    actions
}

/// Converts a byte slice into a vector of little-endian dwords (zero-padded to 4 bytes).
pub fn bytes_to_dwords(bytes: &[u8]) -> Vec<u32> {
    let mut dwords = Vec::with_capacity((bytes.len() + 3) / 4);
    for chunk in bytes.chunks(4) {
        let mut buf = [0u8; 4];
        buf[..chunk.len()].copy_from_slice(chunk);
        dwords.push(u32::from_le_bytes(buf));
    }
    dwords
}

/// Emits the complete sequence of CPString streaming actions to format and write
/// `parts` to `dest_epd`, optionally followed by a `DisplayText` action for `display_str_id`.
pub fn emit_format_parts_actions(
    dest_epd: ValueExpr,
    parts: &[crate::hir::FormatPart],
    display_str_id: Option<u32>,
    always_display: bool,
    var_inits: &std::collections::HashMap<crate::hir::VarId, i32>,
) -> Vec<LirAction> {
    let mut static_bytes = Vec::new();

    for part in parts {
        match part {
            crate::hir::FormatPart::Text(s) => {
                static_bytes.extend_from_slice(s.as_bytes());
            }
            crate::hir::FormatPart::Decimal(expr) => {
                let val = match expr.fold_constants() {
                    crate::hir::HirExpr::Const(c) => c,
                    crate::hir::HirExpr::Var(v) => var_inits.get(&v).copied().unwrap_or(0),
                    _ => 0,
                };
                let s = val.to_string();
                static_bytes.extend_from_slice(s.as_bytes());
            }
            crate::hir::FormatPart::Hex(expr) => {
                let val = match expr.fold_constants() {
                    crate::hir::HirExpr::Const(c) => c as u32,
                    crate::hir::HirExpr::Var(v) => var_inits.get(&v).copied().unwrap_or(0) as u32,
                    _ => 0,
                };
                let s = format!("{:08X}", val);
                static_bytes.extend_from_slice(s.as_bytes());
            }
        }
    }

    let dwords = bytes_to_dwords(&static_bytes);
    let expr_dwords: Vec<ValueExpr> = dwords
        .into_iter()
        .map(|d| ValueExpr::Const(d as i32))
        .collect();
    let mut actions = emit_cpstring_write_block(dest_epd, &expr_dwords, Some(0));

    if let Some(str_id) = display_str_id {
        actions.push(LirAction {
            locid1: ValueExpr::Const(0),
            strid: ValueExpr::Const(str_id as i32),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: ValueExpr::Const(0),
            player2: ValueExpr::Const(0),
            unitid: 0,
            acttype: 9, // DisplayText
            amount: 0,
            flags: if always_display { 4 } else { 0 },
            eudx: 0,
        });
    }

    actions
}
