use crate::chk::Chk;
use crate::error::Result;
use crate::hir::{epd, STRX_BASE_ADDR};
use crate::injector::{pack_chk_action, pack_chk_condition, pack_chk_trigger};
use crate::lir::{LirModule, LirObject, ValueExpr};
use crate::map_context::MapContext;

pub const RUNTIME_MRGN_BASE_ADDR: u32 = 0x0058DC60;

/// Deterministic 64-bit SplitMix64 PRNG for reproducible obscuration passes.
#[derive(Debug, Clone)]
pub struct ObscureRng {
    state: u64,
}

impl ObscureRng {
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x517c_c1b7_2722_0a95
        } else {
            seed
        };
        let mut rng = Self { state };
        rng.next_u64();
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn gen_range(&mut self, start: u32, end_exclusive: u32) -> u32 {
        if start >= end_exclusive {
            return start;
        }
        let range = (end_exclusive - start) as u64;
        let v = self.next_u64() % range;
        start + v as u32
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = (self.next_u64() % ((i + 1) as u64)) as usize;
            slice.swap(i, j);
        }
    }
}

/// Applies random unit-offset bias (`12 * u` compensation) to EPD/death accesses.
/// When `player` is an EPD expression and `unit == 0`, adjusts `player -= 12 * u` and `unit = u`.
pub fn apply_unit_offset_randomization(lir: &mut LirModule, rng: &mut ObscureRng) {
    for obj in lir.objects_mut() {
        if let LirObject::Trigger(trig) = obj {
            for cond in &mut trig.conditions {
                if cond.condtype == 15 && cond.unitid == 0 {
                    let u = rng.gen_range(234, 4096) as u16;
                    if randomize_player_expr(&mut cond.player, u) {
                        cond.unitid = u;
                    }
                }
            }
            for act in &mut trig.actions {
                if act.acttype == 45 && act.unitid == 0 {
                    let u = rng.gen_range(234, 4096) as u16;
                    if randomize_player_expr(&mut act.player1, u) {
                        act.unitid = u;
                    }
                }
                // Strip debug comments in LIR triggers
                if act.acttype == 47 {
                    act.acttype = 0;
                    act.strid = ValueExpr::Const(0);
                }
            }
        }
    }
}

fn randomize_player_expr(expr: &mut ValueExpr, u: u16) -> bool {
    let bias = 12 * (u as i32);
    match expr {
        ValueExpr::Const(p) => {
            if *p == 13 {
                // CurrentPlayer: do not modify register ID
                return false;
            }
            *p = p.wrapping_sub(bias);
            true
        }
        ValueExpr::ObjectEpd { dword_offset, .. } => {
            *dword_offset = dword_offset.wrapping_sub(bias);
            true
        }
        ValueExpr::FieldEpd(fr) => {
            let dword_off = (fr.signed_byte_offset() / 4).wrapping_sub(bias);
            *expr = ValueExpr::ObjectEpd {
                object: fr.object,
                dword_offset: dword_off,
            };
            true
        }
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringRestoreItem {
    pub address: u32,
    pub mask_dwords: Vec<u32>,
    pub orig_dwords: Vec<u32>,
}

/// Scrambles static string literals in `MapContext`'s string table with seeded masking.
/// Returns restore items needed to reconstruct the original plaintext strings in memory at startup
/// via additive decryption triggers, avoiding plaintext string leakage in action fields.
pub fn encrypt_static_strings(
    map_ctx: &mut MapContext,
    rng: &mut ObscureRng,
) -> Vec<StringRestoreItem> {
    let mut restores = Vec::new();
    let tbl = map_ctx.string_table_mut();
    let datatb_len = tbl.datatb().len();

    for data_idx in 0..datatb_len {
        let orig_bytes = tbl.datatb()[data_idx].clone();
        if orig_bytes.is_empty() {
            continue;
        }

        let Some(offset) = tbl.data_index_offset(data_idx) else {
            continue;
        };
        let address = STRX_BASE_ADDR.wrapping_add(offset);

        // Convert original string bytes (including null terminator and 4-byte padding) to dwords
        let padded_len = (orig_bytes.len() + 1 + 3) & !3;
        let mut padded = vec![0u8; padded_len];
        padded[..orig_bytes.len()].copy_from_slice(&orig_bytes);
        padded[orig_bytes.len()] = 0; // null terminator

        let mut orig_dwords = Vec::with_capacity(padded_len / 4);
        for chunk in padded.chunks_exact(4) {
            orig_dwords.push(u32::from_le_bytes(chunk.try_into().unwrap()));
        }

        // Apply seeded additive scrambling to the string bytes in datatb so that
        // adding mask_dwords at runtime will decrypt memory to original plaintext.
        let mut mask_dwords = Vec::new();
        let target_str = &mut tbl.datatb_mut()[data_idx];
        let num_dwords = (orig_bytes.len() + 3) / 4;

        for j in 0..num_dwords {
            if j * 4 + 4 <= orig_bytes.len() {
                let orig_val =
                    u32::from_le_bytes(orig_bytes[j * 4..j * 4 + 4].try_into().unwrap());
                let mask_val = rng.next_u32();
                let scrambled_val = orig_val.wrapping_sub(mask_val);
                target_str[j * 4..j * 4 + 4].copy_from_slice(&scrambled_val.to_le_bytes());
                mask_dwords.push(mask_val);
            } else {
                let k = orig_bytes.len() - j * 4;
                let mut orig_bytes_k = [0u8; 4];
                orig_bytes_k[..k].copy_from_slice(&orig_bytes[j * 4..]);
                let orig_k = u32::from_le_bytes(orig_bytes_k);
                let mask_val = if orig_k > 1 {
                    rng.gen_range(1, orig_k)
                } else {
                    0
                };
                let scrambled_k = orig_k - mask_val;
                target_str[j * 4..].copy_from_slice(&scrambled_k.to_le_bytes()[..k]);
                mask_dwords.push(mask_val);
            }
        }

        restores.push(StringRestoreItem {
            address,
            mask_dwords,
            orig_dwords,
        });
    }

    restores
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrgnRestoreInfo {
    pub mrgn_mask: Vec<u32>,
}

/// Protects CHK sections: scrambles `MRGN`, strips switch names in `SWNM`,
/// and strips debug comment actions (`acttype == 47`) from existing `TRIG` triggers.
pub fn protect_chk_sections(chk: &mut Chk, rng: &mut ObscureRng) -> Result<Option<MrgnRestoreInfo>> {
    let mut mrgn_info = None;

    // 1. Scramble MRGN
    if let Some(mrgn_data) = chk.section(b"MRGN") {
        let dword_count = mrgn_data.len() / 4;
        let mut mrgn_mask = Vec::with_capacity(dword_count);
        for _ in 0..dword_count {
            mrgn_mask.push(rng.next_u32());
        }

        let mut scrambled_mrgn = vec![0u8; mrgn_data.len()];
        for i in 0..dword_count {
            let offset = i * 4;
            let mut orig = u32::from_le_bytes(mrgn_data[offset..offset + 4].try_into().unwrap());
            // Location name string ID is at offset 16 (i % 5 == 4)
            if (offset % 20) == 16 {
                // Clear string ID (lower 16 bits), keep elevation flags (upper 16 bits)
                orig &= 0xFFFF_0000;
            }
            let scrambled = orig.wrapping_sub(mrgn_mask[i]);
            scrambled_mrgn[offset..offset + 4].copy_from_slice(&scrambled.to_le_bytes());
        }

        chk.set_section(b"MRGN", scrambled_mrgn)?;
        mrgn_info = Some(MrgnRestoreInfo { mrgn_mask });
    }

    // 2. Strip SWNM switch names
    if chk.section(b"SWNM").is_some() {
        chk.set_section(b"SWNM", vec![0u8; 1024])?;
    }

    // 3. Strip debug comment actions from existing TRIG section
    if let Some(trig_data) = chk.section(b"TRIG") {
        let mut new_trig = trig_data.to_vec();
        let trigger_count = new_trig.len() / 2400;
        for t_idx in 0..trigger_count {
            let t_base = t_idx * 2400;
            for a_idx in 0..64 {
                let act_base = t_base + 320 + a_idx * 32;
                let acttype = new_trig[act_base + 26];
                if acttype == 47 {
                    // Strip Comment action
                    new_trig[act_base..act_base + 32].fill(0);
                }
            }
        }
        chk.set_section(b"TRIG", new_trig)?;
    }

    Ok(mrgn_info)
}

/// Builds startup restore triggers to insert into `TRIG` before the bootstrap triggers.
/// Restores `MRGN` at `0x0058DC60` and decoded static strings at `0x191943C8` in runtime memory.
pub fn build_obscuration_restore_triggers(
    mrgn_info: Option<&MrgnRestoreInfo>,
    string_restores: &[StringRestoreItem],
) -> Vec<[u8; 2400]> {
    let mut actions = Vec::new();

    // 1. MRGN restore actions (+ mrgn_mask to runtime 0x0058DC60)
    if let Some(mrgn) = mrgn_info {
        for (i, &mask_val) in mrgn.mrgn_mask.iter().enumerate() {
            let target_epd = epd(RUNTIME_MRGN_BASE_ADDR + 4 * (i as u32));
            actions.push(pack_chk_action(
                0,
                0,
                0,
                0,
                target_epd,
                mask_val as i32,
                0,
                45, // SetDeaths
                8,  // Add
                20,
                0,
            ));
        }
    }

    // 2. String restore actions: emit additive decryption triggers (+ mask_val)
    // so that plaintext string bytes are not leaked in the action fields.
    for item in string_restores {
        for (j, &mask_val) in item.mask_dwords.iter().enumerate() {
            let target_epd = epd(item.address + 4 * (j as u32));
            actions.push(pack_chk_action(
                0,
                0,
                0,
                0,
                target_epd,
                mask_val as i32,
                0,
                45, // SetDeaths
                8,  // Add
                20,
                0,
            ));
        }
    }

    if actions.is_empty() {
        return Vec::new();
    }

    // Pack into triggers with guard condition so restore only executes once on tick 0
    let mut triggers = Vec::new();
    let mut act_idx = 0;

    // First trigger: guard on Deaths(11, Exactly, 0, unit=65535) and set to 1
    let guard_cond_0 = pack_chk_condition(0, 11, 0, 65535, 10 /* Exactly */, 15 /* Deaths */, 0, 0, 0);
    let mut first_acts = vec![pack_chk_action(
        0, 0, 0, 0, 11, 1, 65535, 45 /* SetDeaths */, 7 /* SetTo */, 20, 0,
    )];
    let take_first = 63.min(actions.len());
    first_acts.extend_from_slice(&actions[..take_first]);
    act_idx += take_first;

    triggers.push(pack_chk_trigger(&[17], &[guard_cond_0], &first_acts, 0));

    // Subsequent triggers
    let guard_cond_1 = pack_chk_condition(0, 11, 1, 65535, 10 /* Exactly */, 15 /* Deaths */, 0, 0, 0);
    while act_idx < actions.len() {
        let remaining = actions.len() - act_idx;
        let chunk_size = 64.min(remaining);
        let chunk = &actions[act_idx..act_idx + chunk_size];
        act_idx += chunk_size;

        let mut trigger_acts = chunk.to_vec();
        // If this is the last chunk, set guard to 2 so triggers never run again
        if act_idx == actions.len() {
            if trigger_acts.len() < 64 {
                trigger_acts.push(pack_chk_action(
                    0, 0, 0, 0, 11, 2, 65535, 45, 7, 20, 0,
                ));
            }
        }
        triggers.push(pack_chk_trigger(&[17], &[guard_cond_1], &trigger_acts, 0));
    }

    triggers
}
