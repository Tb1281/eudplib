use crate::allocator::AllocatedPayload;
use crate::chk::Chk;
use crate::error::Result;
use crate::hir::{epd, CURRENT_PLAYER_ADDR, PTS_BASE_ADDR, STRX_BASE_ADDR};
use crate::lir::EUDX_SIGNATURE;
use crate::map_context::{MapContext, MapTrigger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationReport {
    pub trigger_count: usize,
    pub byte_buf_count: usize,
    pub payload_offset: usize,
    pub payload_size: usize,
    pub payload_entry_addr: u32,
    pub prt_offsets: Vec<usize>,
    pub ort_offsets: Vec<usize>,
}

/// Statically relocates `payload` against `0x191943C8 + payload_offset`,
/// appends it to `STRx`, and writes the single-stage `TRIG` entry bootstrap
/// while preserving existing classic `TRIG` triggers.
pub fn inject_payload_into_chk(
    chk: &mut Chk,
    map_ctx: &MapContext,
    allocated: &AllocatedPayload,
    trigger_count: usize,
    byte_buf_count: usize,
) -> Result<CompilationReport> {
    inject_payload_into_chk_with_restores(
        chk,
        map_ctx,
        allocated,
        trigger_count,
        byte_buf_count,
        &[],
    )
}

pub fn inject_payload_into_chk_with_restores(
    chk: &mut Chk,
    map_ctx: &MapContext,
    allocated: &AllocatedPayload,
    trigger_count: usize,
    byte_buf_count: usize,
    restore_triggers: &[[u8; 2400]],
) -> Result<CompilationReport> {
    let mut strx_bytes = map_ctx.string_table().serialize_strx();
    let str_padding = (4 - (strx_bytes.len() & 3)) & 3;
    strx_bytes.resize(strx_bytes.len() + str_padding, 0);

    let payload_offset = strx_bytes.len();
    let payload_base_addr = STRX_BASE_ADDR.wrapping_add(payload_offset as u32);
    let payload_entry_addr = payload_base_addr.wrapping_add(allocated.entry_offset);

    let mut relocated = allocated.data.clone();
    let epd_delta = (payload_base_addr / 4) as i32;
    let ptr_delta = payload_base_addr as i32;

    for &prt in &allocated.prt_table {
        let cur = i32::from_le_bytes(relocated[prt..prt + 4].try_into().unwrap());
        let next = cur.wrapping_add(epd_delta);
        relocated[prt..prt + 4].copy_from_slice(&next.to_le_bytes());
    }
    for &ort in &allocated.ort_table {
        let cur = i32::from_le_bytes(relocated[ort..ort + 4].try_into().unwrap());
        let next = cur.wrapping_add(ptr_delta);
        relocated[ort..ort + 4].copy_from_slice(&next.to_le_bytes());
    }

    strx_bytes.extend_from_slice(&relocated);
    chk.set_section(b"STRx", strx_bytes)?;

    let str_bytes = map_ctx.string_table().serialize_str();
    chk.set_section(b"STR ", str_bytes)?;

    let trig_bytes = build_trig_section(payload_entry_addr, map_ctx.triggers(), restore_triggers);
    chk.set_section(b"TRIG", trig_bytes)?;

    chk.fix_map_data();
    chk.optimize();

    Ok(CompilationReport {
        trigger_count,
        byte_buf_count,
        payload_offset,
        payload_size: allocated.data.len(),
        payload_entry_addr,
        prt_offsets: allocated.prt_table.clone(),
        ort_offsets: allocated.ort_table.clone(),
    })
}

pub fn pack_chk_condition(
    locid: u32,
    player: i32,
    amount: u32,
    unitid: u16,
    comparison: u8,
    condtype: u8,
    restype: u8,
    flag: u8,
    eudx: u16,
) -> [u8; 20] {
    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&locid.to_le_bytes());
    out[4..8].copy_from_slice(&(player as u32).to_le_bytes());
    out[8..12].copy_from_slice(&amount.to_le_bytes());
    out[12..14].copy_from_slice(&unitid.to_le_bytes());
    out[14] = comparison;
    out[15] = condtype;
    out[16] = restype;
    out[17] = flag;
    out[18..20].copy_from_slice(&eudx.to_le_bytes());
    out
}

pub fn pack_chk_action(
    locid1: u32,
    strid: u32,
    wavid: u32,
    time: u32,
    player1: i32,
    player2: i32,
    unitid: u16,
    acttype: u8,
    amount: u8,
    flags: u8,
    eudx: u16,
) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0..4].copy_from_slice(&locid1.to_le_bytes());
    out[4..8].copy_from_slice(&strid.to_le_bytes());
    out[8..12].copy_from_slice(&wavid.to_le_bytes());
    out[12..16].copy_from_slice(&time.to_le_bytes());
    out[16..20].copy_from_slice(&(player1 as u32).to_le_bytes());
    out[20..24].copy_from_slice(&(player2 as u32).to_le_bytes());
    out[24..26].copy_from_slice(&unitid.to_le_bytes());
    out[26] = acttype;
    out[27] = amount;
    out[28] = flags;
    out[29] = 0;
    out[30..32].copy_from_slice(&eudx.to_le_bytes());
    out
}

pub fn pack_chk_trigger(
    players: &[u8],
    conditions: &[[u8; 20]],
    actions: &[[u8; 32]],
    flags: u32,
) -> [u8; 2400] {
    let mut out = [0u8; 2400];
    for (i, cond) in conditions.iter().take(16).enumerate() {
        out[i * 20..(i + 1) * 20].copy_from_slice(cond);
    }
    for (i, act) in actions.iter().take(64).enumerate() {
        let base = 320 + i * 32;
        out[base..base + 32].copy_from_slice(act);
    }
    out[2368..2372].copy_from_slice(&flags.to_le_bytes());
    for &p in players {
        if (p as usize) < 28 {
            out[2372 + p as usize] = 1;
        }
    }
    out
}

/// Builds the `TRIG` section: preserves classic map triggers followed by optional
/// obscuration restore triggers and the 70-trigger single-stage `TRIG` entry bootstrap
/// that redirects `pts[player]` (`0x51A280`) into `payload_entry_addr` in `STRx`.
pub fn build_trig_section(
    payload_entry_addr: u32,
    classic_triggers: &[MapTrigger],
    restore_triggers: &[[u8; 2400]],
) -> Vec<u8> {
    let mut out = Vec::with_capacity((classic_triggers.len() + restore_triggers.len() + 70) * 2400);

    // 1. Preserve classic map triggers intact
    for trig in classic_triggers {
        out.extend_from_slice(&trig.raw);
    }

    // 2. Insert obscuration restore triggers
    for trig in restore_triggers {
        out.extend_from_slice(trig);
    }

    let pts_last = PTS_BASE_ADDR + 4; // 0x51A284 (pts[0].lasttrigger)
    let curpl = CURRENT_PLAYER_ADDR; // 0x6509B0

    // 2. Triggers 1..8: set CurrentPlayer = EPD(0x51A284 + 12 * player)
    for player in 0..8u8 {
        let trigger_end = !(pts_last + (player as u32) * 12);
        let act1 = pack_chk_action(
            0,
            0,
            0,
            0,
            epd(curpl),
            epd(pts_last + 12 * (player as u32)),
            0,
            45, // SetDeaths
            7,  // SetTo
            20,
            0,
        );
        let act2 = pack_chk_action(
            0,
            0,
            0,
            0,
            9, // Player 10 (0-based 9)
            trigger_end as i32,
            0,
            45, // SetDeaths
            7,  // SetTo
            20,
            0,
        );
        let trig = pack_chk_trigger(&[player], &[], &[act1, act2], 0);
        out.extend_from_slice(&trig);
    }

    // 3. Triggers 9..38: bit-read pts[player].lasttrigger (bits 2..31) into Deaths(11, unit=0)
    for e in 2..32u32 {
        let bit = 1u32 << e;
        let cond = pack_chk_condition(
            bit,
            13, // CurrentPlayer
            1,
            0,
            0,  // AtLeast
            15, // Deaths
            0,
            0,
            EUDX_SIGNATURE,
        );
        let act = pack_chk_action(
            0, 0, 0, 0, 11, // Player 12 (0-based 11)
            bit as i32, 0, 45, // SetDeaths
            8,  // Add
            20, 0,
        );
        let trig = pack_chk_trigger(&[17], &[cond], &[act], 0);
        out.extend_from_slice(&trig);
    }

    // 4. Trigger 39: initialize Deaths(10) and CurrentPlayer to EPD(4)
    {
        let act1 = pack_chk_action(0, 0, 0, 0, 10, epd(4), 0, 45, 7, 20, 0);
        let act2 = pack_chk_action(0, 0, 0, 0, epd(curpl), epd(4), 0, 45, 7, 20, 0);
        let trig = pack_chk_trigger(&[17], &[], &[act1, act2], 0);
        out.extend_from_slice(&trig);
    }

    // 5. Triggers 40..69: convert byte address in Deaths(11) to EPD(lasttrigger + 4) in CurrentPlayer
    for e in 2..32u32 {
        let bit = 1u32 << e;
        let epd_bit = (1u32 << (e - 2)) as i32;
        let cond = pack_chk_condition(
            bit,
            11, // Player 12 (0-based 11)
            1,
            0,
            0,  // AtLeast
            15, // Deaths
            0,
            0,
            EUDX_SIGNATURE,
        );
        let act1 = pack_chk_action(0, 0, 0, 0, 10, epd_bit, 0, 45, 8, 20, 0);
        let act2 = pack_chk_action(0, 0, 0, 0, epd(curpl), epd_bit, 0, 45, 8, 20, 0);
        let trig = pack_chk_trigger(&[17], &[cond], &[act1, act2], 0);
        out.extend_from_slice(&trig);
    }

    // 6. Trigger 70: patch lasttrigger->nextptr = payload_entry_addr and reset temporary registers
    {
        let mut actions = vec![
            // *(lasttrigger + 4) = payload_entry_addr
            pack_chk_action(0, 0, 0, 0, 13, payload_entry_addr as i32, 0, 45, 7, 20, 0),
            // Clear temporary registers Deaths(9..11) and reset CurrentPlayer to Player 1 (0)
            pack_chk_action(0, 0, 0, 0, 9, 0, 0, 45, 7, 20, 0),
            pack_chk_action(0, 0, 0, 0, 10, 0, 0, 45, 7, 20, 0),
            pack_chk_action(0, 0, 0, 0, 11, 0, 0, 45, 7, 20, 0),
            pack_chk_action(0, 0, 0, 0, epd(curpl), 0, 0, 45, 7, 20, 0),
        ];

        // If there are no classic TRIG triggers to re-run on subsequent ticks, redirect
        // pts[player].firsttrigger (0x51A280 + 12 * p + 8) directly to payload_entry_addr.
        if classic_triggers.is_empty() {
            for player in 0..8u32 {
                let first_trig_addr = PTS_BASE_ADDR + 12 * player + 8;
                actions.push(pack_chk_action(
                    0,
                    0,
                    0,
                    0,
                    epd(first_trig_addr),
                    payload_entry_addr as i32,
                    0,
                    45,
                    7,
                    20,
                    0,
                ));
            }
        }

        let trig = pack_chk_trigger(&[17], &[], &actions, 0);
        out.extend_from_slice(&trig);
    }

    out
}
