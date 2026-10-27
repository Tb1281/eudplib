use crate::error::{EudkitError, Result};
use crate::hir::{
    epd, BlockId, ByteBufId, HirModule, HirStmt, HirTerminator, HirValue, StockAction,
    StockCondition, END_OF_TRIGGERS_SENTINEL,
};
use crate::map_context::MapContext;
use std::collections::HashMap;
use std::fmt::{self, Write};

pub const EUDX_SIGNATURE: u16 = 0x4353; // b"SC"

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CondField {
    LocId,
    Player,
    Amount,
    UnitAndFlags,
    FlagsAndEudx,
}

impl CondField {
    pub const fn byte_offset(self) -> u32 {
        match self {
            Self::LocId => 0,
            Self::Player => 4,
            Self::Amount => 8,
            Self::UnitAndFlags => 12,
            Self::FlagsAndEudx => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActField {
    LocId1,
    StrId,
    WavId,
    Time,
    Player1,
    Player2,
    UnitAndActType,
    FlagsAndEudx,
}

impl ActField {
    pub const fn byte_offset(self) -> u32 {
        match self {
            Self::LocId1 => 0,
            Self::StrId => 4,
            Self::WavId => 8,
            Self::Time => 12,
            Self::Player1 => 16,
            Self::Player2 => 20,
            Self::UnitAndActType => 24,
            Self::FlagsAndEudx => 28,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerSlotField {
    PrevPtr,
    NextPtr,
    Condition(u8, CondField),
    Action(u8, ActField),
    Flags,
    ByteOffset(u32),
}

impl TriggerSlotField {
    pub const fn byte_offset(self) -> u32 {
        match self {
            Self::PrevPtr => 0,
            Self::NextPtr => 4,
            Self::Condition(idx, field) => 8 + (idx as u32) * 20 + field.byte_offset(),
            Self::Action(idx, field) => 8 + 320 + (idx as u32) * 32 + field.byte_offset(),
            Self::Flags => 8 + 320 + 2048,
            Self::ByteOffset(off) => off,
        }
    }
}

/// Typed LIR pointer to a specific field inside a trigger, condition, action slot, or buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldRef {
    pub object: ObjectId,
    pub field: TriggerSlotField,
}

impl FieldRef {
    pub const fn new(object: ObjectId, field: TriggerSlotField) -> Self {
        Self { object, field }
    }

    pub const fn prev_ptr(object_idx: u32) -> Self {
        Self {
            object: ObjectId(object_idx),
            field: TriggerSlotField::PrevPtr,
        }
    }

    pub const fn next_ptr(object_idx: u32) -> Self {
        Self {
            object: ObjectId(object_idx),
            field: TriggerSlotField::NextPtr,
        }
    }

    pub const fn condition(object_idx: u32, slot: u8, field: CondField) -> Self {
        Self {
            object: ObjectId(object_idx),
            field: TriggerSlotField::Condition(slot, field),
        }
    }

    pub const fn action(object_idx: u32, slot: u8, field: ActField) -> Self {
        Self {
            object: ObjectId(object_idx),
            field: TriggerSlotField::Action(slot, field),
        }
    }

    pub const fn flags(object_idx: u32) -> Self {
        Self {
            object: ObjectId(object_idx),
            field: TriggerSlotField::Flags,
        }
    }

    pub const fn byte_offset(self) -> u32 {
        self.field.byte_offset()
    }
}

impl fmt::Display for FieldRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.field {
            TriggerSlotField::PrevPtr => write!(f, "@obj{}.PrevPtr", self.object.0),
            TriggerSlotField::NextPtr => write!(f, "@obj{}.NextPtr", self.object.0),
            TriggerSlotField::Condition(idx, fld) => {
                write!(f, "@obj{}.Condition[{idx}].{fld:?}", self.object.0)
            }
            TriggerSlotField::Action(idx, fld) => {
                write!(f, "@obj{}.Action[{idx}].{fld:?}", self.object.0)
            }
            TriggerSlotField::Flags => write!(f, "@obj{}.Flags", self.object.0),
            TriggerSlotField::ByteOffset(off) => write!(f, "@obj{}+{off}", self.object.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueExpr {
    Const(i32),
    ObjectAddress { object: ObjectId, byte_offset: i32 },
    ObjectEpd { object: ObjectId, dword_offset: i32 },
    FieldAddress(FieldRef),
    FieldEpd(FieldRef),
}

impl fmt::Display for ValueExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(v) => write!(f, "{v}"),
            Self::ObjectAddress {
                object,
                byte_offset,
            } => {
                if *byte_offset == 0 {
                    write!(f, "addr(@obj{})", object.0)
                } else {
                    write!(f, "addr(@obj{}+{})", object.0, byte_offset)
                }
            }
            Self::ObjectEpd {
                object,
                dword_offset,
            } => {
                if *dword_offset == 0 {
                    write!(f, "epd(@obj{})", object.0)
                } else {
                    write!(f, "epd(@obj{}+{}d)", object.0, dword_offset)
                }
            }
            Self::FieldAddress(fr) => write!(f, "addr({fr})"),
            Self::FieldEpd(fr) => write!(f, "epd({fr})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirCondition {
    pub locid: ValueExpr,
    pub player: ValueExpr,
    pub amount: ValueExpr,
    pub unitid: u16,
    pub comparison: u8,
    pub condtype: u8,
    pub restype: u8,
    pub flags: u8,
    pub eudx: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirAction {
    pub locid1: ValueExpr,
    pub strid: ValueExpr,
    pub wavid: ValueExpr,
    pub time: ValueExpr,
    pub player1: ValueExpr,
    pub player2: ValueExpr,
    pub unitid: u16,
    pub acttype: u8,
    pub amount: u8,
    pub flags: u8,
    pub eudx: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirTrigger {
    pub id: ObjectId,
    pub label: Option<String>,
    pub prev_ptr: ValueExpr,
    pub next_ptr: ValueExpr,
    pub conditions: Vec<LirCondition>,
    pub actions: Vec<LirAction>,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirByteBuf {
    pub id: ObjectId,
    pub label: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LirObject {
    Trigger(LirTrigger),
    ByteBuf(LirByteBuf),
}

impl LirObject {
    pub fn id(&self) -> ObjectId {
        match self {
            Self::Trigger(t) => t.id,
            Self::ByteBuf(b) => b.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirModule {
    pub name: String,
    objects: Vec<LirObject>,
    pub entry_trigger: ObjectId,
}

impl LirModule {
    pub fn new(name: impl Into<String>, objects: Vec<LirObject>, entry_trigger: ObjectId) -> Self {
        Self {
            name: name.into(),
            objects,
            entry_trigger,
        }
    }

    pub fn objects(&self) -> &[LirObject] {
        &self.objects
    }

    pub fn objects_mut(&mut self) -> &mut [LirObject] {
        &mut self.objects
    }

    pub fn dump_lir(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "lir_module @{} (entry=@obj{}) {{",
            self.name, self.entry_trigger.0
        );
        for obj in &self.objects {
            match obj {
                LirObject::Trigger(t) => {
                    let lbl = t.label.as_deref().unwrap_or("trig");
                    let _ = writeln!(
                        out,
                        "  object @obj{} [Trigger @{lbl}] (PrevPtr={}, NextPtr={}, Flags={:#x}) {{",
                        t.id.0, t.prev_ptr, t.next_ptr, t.flags
                    );
                    for (idx, c) in t.conditions.iter().enumerate() {
                        let _ = writeln!(
                            out,
                            "    Condition[{idx}]: type={} player={} amount={} unit={} cmp={} loc={} eudx={:#x}",
                            c.condtype, c.player, c.amount, c.unitid, c.comparison, c.locid, c.eudx
                        );
                    }
                    for (idx, a) in t.actions.iter().enumerate() {
                        let _ = writeln!(
                            out,
                            "    Action[{idx}]: type={} p1={} p2={} unit={} mod={} str={} loc={} flags={} eudx={:#x}",
                            a.acttype, a.player1, a.player2, a.unitid, a.amount, a.strid, a.locid1, a.flags, a.eudx
                        );
                    }
                    let _ = writeln!(out, "  }}");
                }
                LirObject::ByteBuf(b) => {
                    let lbl = b.label.as_deref().unwrap_or("buf");
                    let _ = writeln!(
                        out,
                        "  object @obj{} [ByteBuf @{lbl}] (len={}) = {:02X?}",
                        b.id.0,
                        b.data.len(),
                        b.data
                    );
                }
            }
        }
        let _ = writeln!(out, "}}");
        out
    }
}

pub fn lower_hir_to_lir(hir: &HirModule, map_ctx: &mut MapContext) -> Result<LirModule> {
    let main_fn_id = hir
        .main_loop
        .or_else(|| hir.functions.first().map(|f| f.id))
        .ok_or_else(|| EudkitError::Ir("HIR module has no functions".to_string()))?;
    let main_fn = &hir.functions[main_fn_id.0 as usize];
    if main_fn.blocks.is_empty() {
        return Err(EudkitError::Ir(format!(
            "function '{}' has no basic blocks",
            main_fn.name
        )));
    }

    // Reserve ObjectIds for triggers first so Trigger 0 gets @obj0 (predictable FieldRef IDs),
    // or allocate ByteBufs after triggers and remap ByteBufId -> ObjectId.
    // Wait: to resolve HirValue::ByteBufAddress(buf_id, ..) during trigger lowering, we can
    // pre-count how many triggers will be emitted across all blocks, so we know exact ObjectIds
    // for both triggers and ByteBufs up front!
    let mut trigger_count = 0u32;
    let mut block_first_trigger: HashMap<BlockId, ObjectId> = HashMap::new();

    for block in &main_fn.blocks {
        block_first_trigger.insert(block.id, ObjectId(trigger_count));
        let mut block_trigs = 0u32;
        let mut pending_actions = 0usize;

        for stmt in &block.stmts {
            match stmt {
                HirStmt::Trigger { .. } => {
                    if pending_actions > 0 {
                        block_trigs += 1;
                        pending_actions = 0;
                    }
                    block_trigs += 1;
                }
                HirStmt::Action(_) => {
                    pending_actions += 1;
                    if pending_actions == 64 {
                        block_trigs += 1;
                        pending_actions = 0;
                    }
                }
            }
        }
        if pending_actions > 0 {
            block_trigs += 1;
        }
        if matches!(block.terminator, HirTerminator::Branch { .. }) {
            // A Branch terminator emits a reset trigger + conditional branch trigger (2 triggers),
            // or if the block had a preceding trigger with < 64 actions, we still emit a dedicated
            // reset+branch pair so control-flow lowering is clean and self-contained.
            block_trigs += 2;
        } else if block_trigs == 0 {
            // Empty Jump/Return block needs 1 pass-through trigger
            block_trigs += 1;
        }
        trigger_count += block_trigs;
    }

    let mut buf_id_to_obj: HashMap<ByteBufId, ObjectId> = HashMap::new();
    for (idx, buf) in hir.byte_bufs.iter().enumerate() {
        buf_id_to_obj.insert(buf.id, ObjectId(trigger_count + idx as u32));
    }

    let lower_val = |v: HirValue, buf_map: &HashMap<ByteBufId, ObjectId>| -> ValueExpr {
        match v {
            HirValue::Const(c) => ValueExpr::Const(c),
            HirValue::ByteBufAddress(bid, off) => ValueExpr::ObjectAddress {
                object: buf_map[&bid],
                byte_offset: off as i32,
            },
            HirValue::ByteBufEpd(bid, off) => ValueExpr::ObjectEpd {
                object: buf_map[&bid],
                dword_offset: off,
            },
            HirValue::FieldAddress(fr) => ValueExpr::FieldAddress(fr),
            HirValue::FieldEpd(fr) => ValueExpr::FieldEpd(fr),
        }
    };

    let lower_cond = |c: &StockCondition| -> LirCondition {
        match *c {
            StockCondition::Always => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(0),
                amount: ValueExpr::Const(0),
                unitid: 0,
                comparison: 0,
                condtype: 22,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::Never => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(0),
                amount: ValueExpr::Const(0),
                unitid: 0,
                comparison: 0,
                condtype: 23,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::Deaths {
                player,
                comparison,
                amount,
                unit,
            } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(player.encode()),
                amount: ValueExpr::Const(amount as i32),
                unitid: unit,
                comparison: comparison.encode(),
                condtype: 15,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::DeathsX {
                player,
                comparison,
                amount,
                unit,
                mask,
            } => LirCondition {
                locid: ValueExpr::Const(mask as i32),
                player: ValueExpr::Const(player.encode()),
                amount: ValueExpr::Const(amount as i32),
                unitid: unit,
                comparison: comparison.encode(),
                condtype: 15,
                restype: 0,
                flags: 0,
                eudx: EUDX_SIGNATURE,
            },
            StockCondition::Memory {
                address,
                comparison,
                value,
            } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(epd(address)),
                amount: ValueExpr::Const(value as i32),
                unitid: 0,
                comparison: comparison.encode(),
                condtype: 15,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::MemoryX {
                address,
                comparison,
                value,
                mask,
            } => LirCondition {
                locid: ValueExpr::Const(mask as i32),
                player: ValueExpr::Const(epd(address)),
                amount: ValueExpr::Const(value as i32),
                unitid: 0,
                comparison: comparison.encode(),
                condtype: 15,
                restype: 0,
                flags: 0,
                eudx: EUDX_SIGNATURE,
            },
            StockCondition::Accumulate {
                player,
                comparison,
                amount,
                resource,
            } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(player.encode()),
                amount: ValueExpr::Const(amount as i32),
                unitid: 0,
                comparison: comparison.encode(),
                condtype: 4,
                restype: resource.encode(),
                flags: 0,
                eudx: 0,
            },
            StockCondition::Command {
                player,
                comparison,
                amount,
                unit,
            } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(player.encode()),
                amount: ValueExpr::Const(amount as i32),
                unitid: unit,
                comparison: comparison.encode(),
                condtype: 2,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::Bring {
                player,
                comparison,
                amount,
                unit,
                location,
            } => LirCondition {
                locid: ValueExpr::Const(location as i32),
                player: ValueExpr::Const(player.encode()),
                amount: ValueExpr::Const(amount as i32),
                unitid: unit,
                comparison: comparison.encode(),
                condtype: 3,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::Switch { switch, state } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(0),
                amount: ValueExpr::Const(0),
                unitid: 0,
                comparison: state.encode(),
                condtype: 11,
                restype: switch,
                flags: 0,
                eudx: 0,
            },
        }
    };

    let mut lower_act = |a: &StockAction| -> LirAction {
        match a {
            StockAction::SetDeaths {
                player,
                modifier,
                value,
                unit,
            } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(player.encode()),
                player2: lower_val(*value, &buf_id_to_obj),
                unitid: *unit,
                acttype: 45,
                amount: modifier.encode(),
                flags: 20,
                eudx: 0,
            },
            StockAction::SetDeathsX {
                player,
                modifier,
                value,
                unit,
                mask,
            } => LirAction {
                locid1: ValueExpr::Const(*mask as i32),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(player.encode()),
                player2: lower_val(*value, &buf_id_to_obj),
                unitid: *unit,
                acttype: 45,
                amount: modifier.encode(),
                flags: 20,
                eudx: EUDX_SIGNATURE,
            },
            StockAction::SetMemory {
                address,
                modifier,
                value,
            } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(epd(*address)),
                player2: lower_val(*value, &buf_id_to_obj),
                unitid: 0,
                acttype: 45,
                amount: modifier.encode(),
                flags: 20,
                eudx: 0,
            },
            StockAction::SetMemoryX {
                address,
                modifier,
                value,
                mask,
            } => LirAction {
                locid1: ValueExpr::Const(*mask as i32),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(epd(*address)),
                player2: lower_val(*value, &buf_id_to_obj),
                unitid: 0,
                acttype: 45,
                amount: modifier.encode(),
                flags: 20,
                eudx: EUDX_SIGNATURE,
            },
            StockAction::SetField {
                target,
                modifier,
                value,
            } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::FieldEpd(*target),
                player2: lower_val(*value, &buf_id_to_obj),
                unitid: 0,
                acttype: 45,
                amount: modifier.encode(),
                flags: 20,
                eudx: 0,
            },
            StockAction::SetResources {
                player,
                modifier,
                amount,
                resource,
            } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(player.encode()),
                player2: ValueExpr::Const(*amount as i32),
                unitid: resource.encode() as u16,
                acttype: 26,
                amount: modifier.encode(),
                flags: 4,
                eudx: 0,
            },
            StockAction::DisplayText {
                text,
                always_display,
            } => {
                let str_id = map_ctx.get_or_insert_string(text.as_bytes());
                LirAction {
                    locid1: ValueExpr::Const(0),
                    strid: ValueExpr::Const(str_id as i32),
                    wavid: ValueExpr::Const(0),
                    time: ValueExpr::Const(0),
                    player1: ValueExpr::Const(0),
                    player2: ValueExpr::Const(0),
                    unitid: 0,
                    acttype: 9,
                    amount: 0,
                    flags: if *always_display { 4 } else { 0 },
                    eudx: 0,
                }
            }
            StockAction::DisplayTextId {
                string_id,
                always_display,
            } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(*string_id as i32),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(0),
                player2: ValueExpr::Const(0),
                unitid: 0,
                acttype: 9,
                amount: 0,
                flags: if *always_display { 4 } else { 0 },
                eudx: 0,
            },
            StockAction::CreateUnit {
                count,
                unit,
                location,
                player,
            } => LirAction {
                locid1: ValueExpr::Const(*location as i32),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(player.encode()),
                player2: ValueExpr::Const(0),
                unitid: *unit,
                acttype: 44,
                amount: *count,
                flags: 20,
                eudx: 0,
            },
            StockAction::Comment { text } => {
                let str_id = map_ctx.get_or_insert_string(text.as_bytes());
                LirAction {
                    locid1: ValueExpr::Const(0),
                    strid: ValueExpr::Const(str_id as i32),
                    wavid: ValueExpr::Const(0),
                    time: ValueExpr::Const(0),
                    player1: ValueExpr::Const(0),
                    player2: ValueExpr::Const(0),
                    unitid: 0,
                    acttype: 47,
                    amount: 0,
                    flags: 4,
                    eudx: 0,
                }
            }
            StockAction::PreserveTrigger => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(0),
                player2: ValueExpr::Const(0),
                unitid: 0,
                acttype: 3,
                amount: 0,
                flags: 4,
                eudx: 0,
            },
            StockAction::Victory => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(0),
                player2: ValueExpr::Const(0),
                unitid: 0,
                acttype: 1,
                amount: 0,
                flags: 4,
                eudx: 0,
            },
            StockAction::Defeat => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(0),
                player2: ValueExpr::Const(0),
                unitid: 0,
                acttype: 2,
                amount: 0,
                flags: 4,
                eudx: 0,
            },
        }
    };

    let mut objects: Vec<LirObject> = Vec::with_capacity(trigger_count as usize + hir.byte_bufs.len());
    let mut cur_obj_id = 0u32;

    for block in &main_fn.blocks {
        let mut block_triggers: Vec<LirTrigger> = Vec::new();
        let mut pending_actions: Vec<LirAction> = Vec::new();

        let flush_pending = |pending: &mut Vec<LirAction>,
                             out: &mut Vec<LirTrigger>,
                             cur_id: &mut u32,
                             blk_label: &str| {
            if !pending.is_empty() {
                let id = ObjectId(*cur_id);
                *cur_id += 1;
                out.push(LirTrigger {
                    id,
                    label: Some(format!("{blk_label}_coalesced_{}", id.0)),
                    prev_ptr: ValueExpr::Const(0),
                    next_ptr: ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
                    conditions: Vec::new(),
                    actions: std::mem::take(pending),
                    flags: 4,
                });
            }
        };

        for stmt in &block.stmts {
            match stmt {
                HirStmt::Trigger {
                    label,
                    conditions,
                    actions,
                    preserved,
                } => {
                    flush_pending(
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    if conditions.len() > 16 {
                        return Err(EudkitError::Ir(
                            "trigger cannot have more than 16 conditions".to_string(),
                        ));
                    }
                    if actions.len() > 64 {
                        return Err(EudkitError::Ir(
                            "trigger cannot have more than 64 actions".to_string(),
                        ));
                    }
                    let id = ObjectId(cur_obj_id);
                    cur_obj_id += 1;
                    block_triggers.push(LirTrigger {
                        id,
                        label: label.clone().or_else(|| Some(format!("{}_{}", block.label, id.0))),
                        prev_ptr: ValueExpr::Const(0),
                        next_ptr: ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
                        conditions: conditions.iter().map(&lower_cond).collect(),
                        actions: actions.iter().map(&mut lower_act).collect(),
                        flags: if *preserved { 4 } else { 0 },
                    });
                }
                HirStmt::Action(act) => {
                    pending_actions.push(lower_act(act));
                    if pending_actions.len() == 64 {
                        flush_pending(
                            &mut pending_actions,
                            &mut block_triggers,
                            &mut cur_obj_id,
                            &block.label,
                        );
                    }
                }
            }
        }
        flush_pending(
            &mut pending_actions,
            &mut block_triggers,
            &mut cur_obj_id,
            &block.label,
        );

        match &block.terminator {
            HirTerminator::Jump(target_block) => {
                if block_triggers.is_empty() {
                    let id = ObjectId(cur_obj_id);
                    cur_obj_id += 1;
                    block_triggers.push(LirTrigger {
                        id,
                        label: Some(format!("{}_jump", block.label)),
                        prev_ptr: ValueExpr::Const(0),
                        next_ptr: ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
                        conditions: Vec::new(),
                        actions: Vec::new(),
                        flags: 4,
                    });
                }
                for i in 0..block_triggers.len() - 1 {
                    let next_id = block_triggers[i + 1].id;
                    block_triggers[i].next_ptr = ValueExpr::ObjectAddress {
                        object: next_id,
                        byte_offset: 0,
                    };
                }
                let target_obj = block_first_trigger[target_block];
                block_triggers.last_mut().unwrap().next_ptr = ValueExpr::ObjectAddress {
                    object: target_obj,
                    byte_offset: 0,
                };
            }
            HirTerminator::Return => {
                if block_triggers.is_empty() {
                    let id = ObjectId(cur_obj_id);
                    cur_obj_id += 1;
                    block_triggers.push(LirTrigger {
                        id,
                        label: Some(format!("{}_ret", block.label)),
                        prev_ptr: ValueExpr::Const(0),
                        next_ptr: ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
                        conditions: Vec::new(),
                        actions: Vec::new(),
                        flags: 4,
                    });
                }
                for i in 0..block_triggers.len() - 1 {
                    let next_id = block_triggers[i + 1].id;
                    block_triggers[i].next_ptr = ValueExpr::ObjectAddress {
                        object: next_id,
                        byte_offset: 0,
                    };
                }
                block_triggers.last_mut().unwrap().next_ptr =
                    ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32);
            }
            HirTerminator::Branch {
                conditions,
                then_block,
                else_block,
            } => {
                let reset_id = ObjectId(cur_obj_id);
                cur_obj_id += 1;
                let branch_id = ObjectId(cur_obj_id);
                cur_obj_id += 1;

                let then_obj = block_first_trigger[then_block];
                let else_obj = block_first_trigger[else_block];
                let branch_next_field = FieldRef::new(branch_id, TriggerSlotField::NextPtr);

                let reset_trig = LirTrigger {
                    id: reset_id,
                    label: Some(format!("{}_branch_reset", block.label)),
                    prev_ptr: ValueExpr::Const(0),
                    next_ptr: ValueExpr::ObjectAddress {
                        object: branch_id,
                        byte_offset: 0,
                    },
                    conditions: Vec::new(),
                    actions: vec![LirAction {
                        locid1: ValueExpr::Const(0),
                        strid: ValueExpr::Const(0),
                        wavid: ValueExpr::Const(0),
                        time: ValueExpr::Const(0),
                        player1: ValueExpr::FieldEpd(branch_next_field),
                        player2: ValueExpr::ObjectAddress {
                            object: else_obj,
                            byte_offset: 0,
                        },
                        unitid: 0,
                        acttype: 45,
                        amount: 7, // SetTo
                        flags: 20,
                        eudx: 0,
                    }],
                    flags: 4,
                };

                let branch_trig = LirTrigger {
                    id: branch_id,
                    label: Some(format!("{}_branch_cond", block.label)),
                    prev_ptr: ValueExpr::Const(0),
                    next_ptr: ValueExpr::ObjectAddress {
                        object: else_obj,
                        byte_offset: 0,
                    },
                    conditions: conditions.iter().map(&lower_cond).collect(),
                    actions: vec![LirAction {
                        locid1: ValueExpr::Const(0),
                        strid: ValueExpr::Const(0),
                        wavid: ValueExpr::Const(0),
                        time: ValueExpr::Const(0),
                        player1: ValueExpr::FieldEpd(branch_next_field),
                        player2: ValueExpr::ObjectAddress {
                            object: then_obj,
                            byte_offset: 0,
                        },
                        unitid: 0,
                        acttype: 45,
                        amount: 7, // SetTo
                        flags: 20,
                        eudx: 0,
                    }],
                    flags: 4,
                };

                block_triggers.push(reset_trig);
                block_triggers.push(branch_trig);

                for i in 0..block_triggers.len() - 1 {
                    let next_id = block_triggers[i + 1].id;
                    block_triggers[i].next_ptr = ValueExpr::ObjectAddress {
                        object: next_id,
                        byte_offset: 0,
                    };
                }
            }
        }

        for t in block_triggers {
            objects.push(LirObject::Trigger(t));
        }
    }

    for buf in &hir.byte_bufs {
        let obj_id = buf_id_to_obj[&buf.id];
        objects.push(LirObject::ByteBuf(LirByteBuf {
            id: obj_id,
            label: Some(buf.name.clone()),
            data: buf.data.clone(),
        }));
    }

    let entry_trigger = block_first_trigger[&main_fn.blocks[0].id];
    Ok(LirModule::new(&hir.name, objects, entry_trigger))
}
