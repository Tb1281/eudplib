//! Canonical byte layouts for CHK triggers emitted by the Backend.
//!
//! Two trigger layouts exist:
//!
//! * [`classic`] — the 2400-byte StarCraft `TRIG` entry used by the Map Context
//!   parser, the obscuration pass, and the `TRIG` bootstrap. It has no
//!   `prev_ptr`/`next_ptr` prefix because the engine links entries through the
//!   player trigger table.
//! * [`payload`] — the 2408-byte trigger stored in the EUD Payload. It adds
//!   `prev_ptr`/`next_ptr` before the condition slots and a trailing
//!   `currentAction` byte.
//!
//! Both layouts share 16 condition slots of 20 bytes and 64 action slots of
//! 32 bytes, so their slot offsets differ only by the payload's 8-byte prefix.

/// Maximum number of condition slots in a trigger.
pub const CONDITION_COUNT: usize = 16;
/// Size of one condition slot in bytes.
pub const CONDITION_SIZE: usize = 20;
/// Maximum number of action slots in a trigger.
pub const ACTION_COUNT: usize = 64;
/// Size of one action slot in bytes.
pub const ACTION_SIZE: usize = 32;
/// Number of player slot bytes in a classic `TRIG` entry.
pub const PLAYER_COUNT: usize = 28;
/// Size of the trigger flags dword in bytes.
pub const FLAGS_SIZE: usize = 4;

/// Offset of the `condtype` byte within a condition slot.
pub const CONDITION_TYPE_OFFSET: usize = 15;
/// Offset of the `acttype` byte within an action slot.
pub const ACTION_TYPE_OFFSET: usize = 26;

/// Condition slot size in dwords, as used by allocator occupation maps.
pub const CONDITION_DWORDS: usize = CONDITION_SIZE / 4;
/// Action slot size in dwords, as used by allocator occupation maps.
pub const ACTION_DWORDS: usize = ACTION_SIZE / 4;

/// Packs one condition slot. Condition slots are identical in both layouts.
#[allow(clippy::too_many_arguments)] // Mirrors the CHK condition slot field order.
pub fn pack_condition(
    locid: u32,
    player: i32,
    amount: u32,
    unitid: u16,
    comparison: u8,
    condtype: u8,
    restype: u8,
    flag: u8,
    eudx: u16,
) -> [u8; CONDITION_SIZE] {
    let mut out = [0u8; CONDITION_SIZE];
    out[0..4].copy_from_slice(&locid.to_le_bytes());
    out[4..8].copy_from_slice(&(player as u32).to_le_bytes());
    out[8..12].copy_from_slice(&amount.to_le_bytes());
    out[12..14].copy_from_slice(&unitid.to_le_bytes());
    out[14] = comparison;
    out[CONDITION_TYPE_OFFSET] = condtype;
    out[16] = restype;
    out[17] = flag;
    out[18..20].copy_from_slice(&eudx.to_le_bytes());
    out
}

/// Packs one action slot. Action slots are identical in both layouts.
#[allow(clippy::too_many_arguments)] // Mirrors the CHK action slot field order.
pub fn pack_action(
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
) -> [u8; ACTION_SIZE] {
    let mut out = [0u8; ACTION_SIZE];
    out[0..4].copy_from_slice(&locid1.to_le_bytes());
    out[4..8].copy_from_slice(&strid.to_le_bytes());
    out[8..12].copy_from_slice(&wavid.to_le_bytes());
    out[12..16].copy_from_slice(&time.to_le_bytes());
    out[16..20].copy_from_slice(&(player1 as u32).to_le_bytes());
    out[20..24].copy_from_slice(&(player2 as u32).to_le_bytes());
    out[24..26].copy_from_slice(&unitid.to_le_bytes());
    out[ACTION_TYPE_OFFSET] = acttype;
    out[27] = amount;
    out[28] = flags;
    out[29] = 0;
    out[30..32].copy_from_slice(&eudx.to_le_bytes());
    out
}

fn copy_conditions(out: &mut [u8], base: usize, conditions: &[[u8; CONDITION_SIZE]]) {
    for (index, condition) in conditions.iter().take(CONDITION_COUNT).enumerate() {
        let offset = base + index * CONDITION_SIZE;
        out[offset..offset + CONDITION_SIZE].copy_from_slice(condition);
    }
}

fn copy_actions(out: &mut [u8], base: usize, actions: &[[u8; ACTION_SIZE]]) {
    for (index, action) in actions.iter().take(ACTION_COUNT).enumerate() {
        let offset = base + index * ACTION_SIZE;
        out[offset..offset + ACTION_SIZE].copy_from_slice(action);
    }
}

/// The classic StarCraft `TRIG` section entry layout.
pub mod classic {
    use super::{
        copy_actions, copy_conditions, ACTION_COUNT, ACTION_SIZE, CONDITION_COUNT, CONDITION_SIZE,
        FLAGS_SIZE, PLAYER_COUNT,
    };

    pub use super::{pack_action, pack_condition, ACTION_TYPE_OFFSET, CONDITION_TYPE_OFFSET};

    /// Size of one classic `TRIG` entry in bytes.
    pub const TRIGGER_SIZE: usize = 2400;
    /// Offset of the first condition slot.
    pub const CONDITIONS_OFFSET: usize = 0;
    /// Offset of the first action slot: 16 conditions of 20 bytes.
    pub const ACTIONS_OFFSET: usize = CONDITIONS_OFFSET + CONDITION_COUNT * CONDITION_SIZE;
    /// Offset of the trigger flags dword.
    pub const FLAGS_OFFSET: usize = ACTIONS_OFFSET + ACTION_COUNT * ACTION_SIZE;
    /// Offset of the 28 player slot bytes.
    pub const PLAYERS_OFFSET: usize = FLAGS_OFFSET + FLAGS_SIZE;
    /// End of the player slot bytes, which is also the end of the entry.
    pub const PLAYERS_END: usize = PLAYERS_OFFSET + PLAYER_COUNT;

    const _: () = assert!(PLAYERS_END == TRIGGER_SIZE);

    /// Offset of condition slot `index` within a classic `TRIG` entry.
    pub const fn condition_offset(index: usize) -> usize {
        CONDITIONS_OFFSET + index * CONDITION_SIZE
    }

    /// Offset of action slot `index` within a classic `TRIG` entry.
    pub const fn action_offset(index: usize) -> usize {
        ACTIONS_OFFSET + index * ACTION_SIZE
    }

    /// Packs one classic `TRIG` entry. Slots beyond the layout's fixed capacity
    /// are ignored and missing slots stay zeroed.
    pub fn pack_trigger(
        players: &[u8],
        conditions: &[[u8; CONDITION_SIZE]],
        actions: &[[u8; ACTION_SIZE]],
        flags: u32,
    ) -> [u8; TRIGGER_SIZE] {
        let mut out = [0u8; TRIGGER_SIZE];
        copy_conditions(&mut out, CONDITIONS_OFFSET, conditions);
        copy_actions(&mut out, ACTIONS_OFFSET, actions);
        out[FLAGS_OFFSET..FLAGS_OFFSET + FLAGS_SIZE].copy_from_slice(&flags.to_le_bytes());
        for &player in players {
            if (player as usize) < PLAYER_COUNT {
                out[PLAYERS_OFFSET + player as usize] = 1;
            }
        }
        out
    }

    /// Borrowed view over one classic `TRIG` entry.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TriggerView<'a> {
        raw: &'a [u8],
    }

    impl<'a> TriggerView<'a> {
        pub fn new(raw: &'a [u8]) -> Self {
            assert_eq!(
                raw.len(),
                TRIGGER_SIZE,
                "classic TRIG entry must be {TRIGGER_SIZE} bytes"
            );
            Self { raw }
        }

        pub fn raw(&self) -> &'a [u8] {
            self.raw
        }

        pub fn flags(&self) -> u32 {
            u32::from_le_bytes(
                self.raw[FLAGS_OFFSET..FLAGS_OFFSET + FLAGS_SIZE]
                    .try_into()
                    .unwrap(),
            )
        }

        pub fn players(&self) -> [u8; PLAYER_COUNT] {
            let mut players = [0u8; PLAYER_COUNT];
            players.copy_from_slice(&self.raw[PLAYERS_OFFSET..PLAYERS_END]);
            players
        }

        pub fn condition(&self, index: usize) -> &'a [u8; CONDITION_SIZE] {
            let offset = condition_offset(index);
            self.raw[offset..offset + CONDITION_SIZE]
                .try_into()
                .unwrap()
        }

        pub fn action(&self, index: usize) -> &'a [u8; ACTION_SIZE] {
            let offset = action_offset(index);
            self.raw[offset..offset + ACTION_SIZE].try_into().unwrap()
        }

        pub fn condition_type(&self, index: usize) -> u8 {
            self.raw[condition_offset(index) + CONDITION_TYPE_OFFSET]
        }

        pub fn action_type(&self, index: usize) -> u8 {
            self.raw[action_offset(index) + ACTION_TYPE_OFFSET]
        }

        /// Number of leading non-empty condition slots.
        pub fn condition_count(&self) -> usize {
            (0..CONDITION_COUNT)
                .take_while(|&index| self.condition_type(index) != 0)
                .count()
        }

        /// Number of leading non-empty action slots.
        pub fn action_count(&self) -> usize {
            (0..ACTION_COUNT)
                .take_while(|&index| self.action_type(index) != 0)
                .count()
        }
    }

    /// Unpacks one classic `TRIG` entry for field access.
    pub fn unpack_trigger(raw: &[u8]) -> TriggerView<'_> {
        TriggerView::new(raw)
    }
}

/// The in-memory trigger layout emitted into the EUD Payload.
pub mod payload {
    use super::{
        copy_actions, copy_conditions, ACTION_COUNT, ACTION_SIZE, ACTION_TYPE_OFFSET,
        CONDITION_COUNT, CONDITION_SIZE, CONDITION_TYPE_OFFSET, FLAGS_SIZE,
    };

    pub use super::{pack_action, pack_condition};

    /// Size of one Payload trigger in bytes.
    pub const TRIGGER_SIZE: usize = 2408;
    /// Offset of the `prev_ptr` field.
    pub const PREV_PTR_OFFSET: usize = 0;
    /// Offset of the `next_ptr` field.
    pub const NEXT_PTR_OFFSET: usize = PREV_PTR_OFFSET + 4;
    /// Offset of the first condition slot, after the two pointer fields.
    pub const CONDITIONS_OFFSET: usize = NEXT_PTR_OFFSET + 4;
    /// Offset of the first action slot: 16 conditions of 20 bytes.
    pub const ACTIONS_OFFSET: usize = CONDITIONS_OFFSET + CONDITION_COUNT * CONDITION_SIZE;
    /// Offset of the trigger flags dword.
    pub const FLAGS_OFFSET: usize = ACTIONS_OFFSET + ACTION_COUNT * ACTION_SIZE;
    /// Offset of the trailing `currentAction` byte.
    pub const CURRENT_ACTION_OFFSET: usize = TRIGGER_SIZE - 1;
    /// Unused bytes between the flags dword and `currentAction`.
    pub const TRAILING_UNUSED_BYTES: usize = CURRENT_ACTION_OFFSET - (FLAGS_OFFSET + FLAGS_SIZE);

    const _: () = assert!(TRAILING_UNUSED_BYTES == 27);
    const _: () = assert!(TRIGGER_SIZE.is_multiple_of(4));

    /// Offset of condition slot `index` within a Payload trigger.
    pub const fn condition_offset(index: usize) -> usize {
        CONDITIONS_OFFSET + index * CONDITION_SIZE
    }

    /// Offset of action slot `index` within a Payload trigger.
    pub const fn action_offset(index: usize) -> usize {
        ACTIONS_OFFSET + index * ACTION_SIZE
    }

    /// Packs one Payload trigger from already-relocated pointer values and raw
    /// condition/action slots. Slots beyond the fixed capacity are ignored.
    pub fn pack_trigger(
        prev_ptr: i32,
        next_ptr: i32,
        conditions: &[[u8; CONDITION_SIZE]],
        actions: &[[u8; ACTION_SIZE]],
        flags: u32,
        current_action: u8,
    ) -> [u8; TRIGGER_SIZE] {
        let mut out = [0u8; TRIGGER_SIZE];
        out[PREV_PTR_OFFSET..PREV_PTR_OFFSET + 4].copy_from_slice(&prev_ptr.to_le_bytes());
        out[NEXT_PTR_OFFSET..NEXT_PTR_OFFSET + 4].copy_from_slice(&next_ptr.to_le_bytes());
        copy_conditions(&mut out, CONDITIONS_OFFSET, conditions);
        copy_actions(&mut out, ACTIONS_OFFSET, actions);
        out[FLAGS_OFFSET..FLAGS_OFFSET + FLAGS_SIZE].copy_from_slice(&flags.to_le_bytes());
        out[CURRENT_ACTION_OFFSET] = current_action;
        out
    }

    /// Borrowed view over one Payload trigger.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TriggerView<'a> {
        raw: &'a [u8],
    }

    impl<'a> TriggerView<'a> {
        pub fn new(raw: &'a [u8]) -> Self {
            assert_eq!(
                raw.len(),
                TRIGGER_SIZE,
                "Payload trigger must be {TRIGGER_SIZE} bytes"
            );
            Self { raw }
        }

        pub fn raw(&self) -> &'a [u8] {
            self.raw
        }

        pub fn prev_ptr(&self) -> i32 {
            i32::from_le_bytes(
                self.raw[PREV_PTR_OFFSET..PREV_PTR_OFFSET + 4]
                    .try_into()
                    .unwrap(),
            )
        }

        pub fn next_ptr(&self) -> i32 {
            i32::from_le_bytes(
                self.raw[NEXT_PTR_OFFSET..NEXT_PTR_OFFSET + 4]
                    .try_into()
                    .unwrap(),
            )
        }

        pub fn flags(&self) -> u32 {
            u32::from_le_bytes(
                self.raw[FLAGS_OFFSET..FLAGS_OFFSET + FLAGS_SIZE]
                    .try_into()
                    .unwrap(),
            )
        }

        pub fn current_action(&self) -> u8 {
            self.raw[CURRENT_ACTION_OFFSET]
        }

        pub fn condition(&self, index: usize) -> &'a [u8; CONDITION_SIZE] {
            let offset = condition_offset(index);
            self.raw[offset..offset + CONDITION_SIZE]
                .try_into()
                .unwrap()
        }

        pub fn action(&self, index: usize) -> &'a [u8; ACTION_SIZE] {
            let offset = action_offset(index);
            self.raw[offset..offset + ACTION_SIZE].try_into().unwrap()
        }

        pub fn condition_type(&self, index: usize) -> u8 {
            self.raw[condition_offset(index) + CONDITION_TYPE_OFFSET]
        }

        pub fn action_type(&self, index: usize) -> u8 {
            self.raw[action_offset(index) + ACTION_TYPE_OFFSET]
        }

        /// Number of leading non-empty condition slots.
        pub fn condition_count(&self) -> usize {
            (0..CONDITION_COUNT)
                .take_while(|&index| self.condition_type(index) != 0)
                .count()
        }

        /// Number of leading non-empty action slots.
        pub fn action_count(&self) -> usize {
            (0..ACTION_COUNT)
                .take_while(|&index| self.action_type(index) != 0)
                .count()
        }
    }

    /// Unpacks one Payload trigger for field access.
    pub fn unpack_trigger(raw: &[u8]) -> TriggerView<'_> {
        TriggerView::new(raw)
    }
}
