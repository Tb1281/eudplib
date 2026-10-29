use crate::allocator::AllocatedPayload;
use crate::chk::Chk;
use crate::chk_layout::classic;
use crate::error::Result;
use crate::hir::{epd, CURRENT_PLAYER_ADDR, PTS_BASE_ADDR, STRX_BASE_ADDR};
use crate::lir::EUDX_SIGNATURE;
use crate::map_context::{MapData, MapTrigger};

pub use crate::chk_layout::classic::{
    pack_action as pack_chk_action, pack_condition as pack_chk_condition,
    pack_trigger as pack_chk_trigger,
};

/// Number of classic `TRIG` entries in the single-stage entry bootstrap.
pub const BOOTSTRAP_TRIGGER_COUNT: usize = 70;

/// The typed result of one compilation, shared by the Rust CLI, the Python
/// runner, and the Python CLI.
///
/// `Serialize`/`Deserialize` are the wire format for the report that the
/// Python runner hands back to the Rust CLI. Every field is required: a
/// missing field is a malformed report, never an implicit zero.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompilationReport {
    pub trigger_count: usize,
    pub byte_buf_count: usize,
    /// Number of `VProc` slots in the allocated `LIR` module.
    pub vproc_count: usize,
    pub payload_offset: usize,
    pub payload_size: usize,
    pub payload_entry_addr: u32,
    pub prt_offsets: Vec<usize>,
    pub ort_offsets: Vec<usize>,
}

impl CompilationReport {
    /// Renders the one-line build summary printed by every CLI.
    pub fn summary(&self, output_map: impl std::fmt::Display) -> String {
        format!(
            "Built {} (triggers={}, payload_bytes={}, entry={:#010X})",
            output_map, self.trigger_count, self.payload_size, self.payload_entry_addr
        )
    }

    /// Serializes this report to the JSON wire format shared with Python.
    pub fn to_json(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parses the JSON wire format written by the Python runner.
    pub fn from_json(text: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// Statically relocates `payload` against `0x191943C8 + payload_offset`,
/// appends it to `STRx`, and writes the single-stage `TRIG` entry bootstrap
/// while preserving existing classic `TRIG` triggers.
pub fn inject_payload_into_chk(
    chk: &mut Chk,
    map_data: &MapData,
    allocated: &AllocatedPayload,
    trigger_count: usize,
    byte_buf_count: usize,
    vproc_count: usize,
) -> Result<CompilationReport> {
    inject_payload_into_chk_with_restores(
        chk,
        map_data,
        allocated,
        trigger_count,
        byte_buf_count,
        vproc_count,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn inject_payload_into_chk_with_restores(
    chk: &mut Chk,
    map_data: &MapData,
    allocated: &AllocatedPayload,
    trigger_count: usize,
    byte_buf_count: usize,
    vproc_count: usize,
    restore_triggers: &[[u8; classic::TRIGGER_SIZE]],
) -> Result<CompilationReport> {
    let mut strx_bytes = map_data.string_table().serialize_strx();
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

    let str_bytes = map_data.string_table().serialize_str();
    chk.set_section(b"STR ", str_bytes)?;

    let trig_bytes = build_trig_section(payload_entry_addr, map_data.triggers(), restore_triggers);
    chk.set_section(b"TRIG", trig_bytes)?;

    chk.fix_map_data();
    chk.optimize();

    Ok(CompilationReport {
        trigger_count,
        byte_buf_count,
        vproc_count,
        payload_offset,
        payload_size: allocated.data.len(),
        payload_entry_addr,
        prt_offsets: allocated.prt_table.clone(),
        ort_offsets: allocated.ort_table.clone(),
    })
}

/// Builds the `TRIG` section: preserves classic map triggers followed by optional
/// obscuration restore triggers and the 70-trigger single-stage `TRIG` entry bootstrap
/// that redirects `pts[player]` (`0x51A280`) into `payload_entry_addr` in `STRx`.
pub fn build_trig_section(
    payload_entry_addr: u32,
    classic_triggers: &[MapTrigger],
    restore_triggers: &[[u8; classic::TRIGGER_SIZE]],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        (classic_triggers.len() + restore_triggers.len() + BOOTSTRAP_TRIGGER_COUNT)
            * classic::TRIGGER_SIZE,
    );

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
