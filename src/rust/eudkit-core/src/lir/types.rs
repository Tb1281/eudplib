use crate::hir::{
    BlockId, HirExpr, HirModule, Modifier, StockAction, StockCondition, VarId,
};
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
pub enum VProcSlotField {
    TriggerBase,
    NextPtr,
    Mask,
    Dest,
    Value,
    Modifier,
}

impl VProcSlotField {
    pub const fn signed_byte_offset(self, slot_idx: u32) -> i32 {
        let s = (slot_idx as i32) * 72;
        match self {
            Self::TriggerBase => s - 4,
            Self::NextPtr => s,
            Self::Mask => 324 + s,
            Self::Dest => 324 + s + 16,
            Self::Value => 324 + s + 20,
            Self::Modifier => 324 + s + 24,
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
    VProcSlot(u32, VProcSlotField),
    ByteOffset(u32),
}

impl TriggerSlotField {
    pub const fn signed_byte_offset(self) -> i32 {
        match self {
            Self::PrevPtr => 0,
            Self::NextPtr => 4,
            Self::Condition(idx, field) => (8 + (idx as u32) * 20 + field.byte_offset()) as i32,
            Self::Action(idx, field) => {
                (8 + 320 + (idx as u32) * 32 + field.byte_offset()) as i32
            }
            Self::Flags => (8 + 320 + 2048) as i32,
            Self::VProcSlot(slot, fld) => fld.signed_byte_offset(slot),
            Self::ByteOffset(off) => off as i32,
        }
    }

    pub const fn byte_offset(self) -> u32 {
        self.signed_byte_offset() as u32
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

    pub const fn vproc_slot(object_idx: u32, slot: u32, field: VProcSlotField) -> Self {
        Self {
            object: ObjectId(object_idx),
            field: TriggerSlotField::VProcSlot(slot, field),
        }
    }

    pub const fn signed_byte_offset(self) -> i32 {
        self.field.signed_byte_offset()
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
            TriggerSlotField::VProcSlot(slot, fld) => {
                write!(f, "@obj{}.VProc[{slot}].{fld:?}", self.object.0)
            }
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
pub struct LirVProcSlot {
    pub label: Option<String>,
    pub mask: ValueExpr,
    pub dest: ValueExpr,
    pub value: ValueExpr,
    pub modifier: Modifier,
    pub next_ptr: ValueExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirVProcBuffer {
    pub id: ObjectId,
    pub slots: Vec<LirVProcSlot>,
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
    VProcBuffer(LirVProcBuffer),
    ByteBuf(LirByteBuf),
}

impl LirObject {
    pub fn id(&self) -> ObjectId {
        match self {
            Self::Trigger(t) => t.id,
            Self::VProcBuffer(vb) => vb.id,
            Self::ByteBuf(b) => b.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LirModule {
    pub name: String,
    pub(crate) objects: Vec<LirObject>,
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

    pub fn vproc_count(&self) -> usize {
        self.objects
            .iter()
            .map(|o| match o {
                LirObject::VProcBuffer(vb) => vb.slots.len(),
                _ => 0,
            })
            .sum()
    }

    pub fn trigger_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| matches!(o, LirObject::Trigger(_)))
            .count()
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
                    let lbl = t.label.as_deref().unwrap_or("anon");
                    let _ = writeln!(
                        out,
                        "  object @obj{} [Trigger @{lbl}] (flags={}, next={}, prev={}) {{",
                        t.id.0, t.flags, t.next_ptr, t.prev_ptr
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
                LirObject::VProcBuffer(vb) => {
                    let _ = writeln!(
                        out,
                        "  object @obj{} [VProcBuffer] (slots={}) {{",
                        vb.id.0,
                        vb.slots.len()
                    );
                    for (idx, s) in vb.slots.iter().enumerate() {
                        let lbl = s.label.as_deref().unwrap_or("vproc");
                        let _ = writeln!(
                            out,
                            "    Slot[{idx}] (@{lbl}): mask={} dest={} val={} mod={} next={}",
                            s.mask, s.dest, s.value, s.modifier, s.next_ptr
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

#[derive(Debug, Clone)]
pub struct ScratchAllocator {
    pool: Vec<VarId>,
    in_use: Vec<bool>,
}

impl ScratchAllocator {
    pub fn new() -> Self {
        Self {
            pool: Vec::new(),
            in_use: Vec::new(),
        }
    }

    pub fn acquire(&mut self, hir: &mut HirModule) -> VarId {
        for (i, used) in self.in_use.iter_mut().enumerate() {
            if !*used {
                *used = true;
                return self.pool[i];
            }
        }
        let idx = self.pool.len();
        let vid = hir.add_var(format!("__scratch_{idx}"), 0);
        self.pool.push(vid);
        self.in_use.push(true);
        vid
    }

    pub fn release(&mut self, var: VarId) {
        for (i, &v) in self.pool.iter().enumerate() {
            if v == var {
                self.in_use[i] = false;
                return;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExpandedOp {
    LeafAction(StockAction),
    Trigger {
        label: Option<String>,
        conditions: Vec<StockCondition>,
        actions: Vec<StockAction>,
        preserved: bool,
    },
    VProcCopyVar {
        src: VarId,
        dst: VarId,
        modifier: Modifier,
        mask: Option<u32>,
    },
    VProcBitAndVar {
        src: VarId,
        dst: VarId,
    },
    VProcBitOrVar {
        src: VarId,
        dst: VarId,
    },
    VProcBitXorVar {
        src: VarId,
        dst: VarId,
    },
    DynamicShl {
        dst: VarId,
        shift: VarId,
    },
    DynamicShr {
        dst: VarId,
        shift: VarId,
        pow2_scratch: VarId,
        div_x_scratch: VarId,
        div_q_scratch: VarId,
    },
    DynamicMul {
        lhs: VarId,
        rhs: VarId,
        dst: VarId,
    },
    DynamicDivRem {
        dividend: VarId,
        divisor: VarId,
        quot_dst: VarId,
        div_x_scratch: VarId,
    },
    PatchNextTriggerCond {
        src: VarId,
        cond_idx: u8,
        field: CondField,
        is_epd: bool,
    },
    PatchNextTriggerAct {
        src: VarId,
        act_idx: u8,
        field: ActField,
        is_epd: bool,
    },
    CallSetup {
        callee: crate::hir::FuncId,
        rets: Vec<VarId>,
        return_block: BlockId,
    },
    CallIndirectSetup {
        ptr: crate::hir::FuncPtrId,
        rets: Vec<VarId>,
        return_block: BlockId,
    },
    WriteFuncReturnSlot {
        func: crate::hir::FuncId,
        slot_idx: u32,
        src_var: Option<VarId>,
        const_val: Option<i32>,
    },
    FuncPtrAssignTarget {
        ptr: crate::hir::FuncPtrId,
        target: crate::hir::FuncId,
    },
    MemReadSetup {
        dst: VarId,
        epd: HirExpr,
        mask: u32,
        shift: i32,
        initial_val: i32,
        return_block: BlockId,
    },
}

#[derive(Debug, Clone)]
pub enum ExpandedTerminator {
    Jump(BlockId),
    Branch {
        conditions: Vec<StockCondition>,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return,
    FunctionReturn(crate::hir::FuncId),
    ReadTableJump {
        mask: u32,
        shift: i32,
    },
}

#[derive(Debug, Clone)]
pub struct ExpandedBlock {
    pub id: BlockId,
    pub label: String,
    pub ops: Vec<ExpandedOp>,
    pub terminator: ExpandedTerminator,
}

#[derive(Debug, Clone)]
pub struct ExpandedCfg {
    pub blocks: Vec<ExpandedBlock>,
    pub fn_entry_block: HashMap<crate::hir::FuncId, BlockId>,
    pub fp_trampoline_block: HashMap<crate::hir::FuncPtrId, BlockId>,
}

pub fn alloc_block(out_blocks: &mut Vec<ExpandedBlock>, label: impl Into<String>) -> BlockId {
    let id = BlockId(out_blocks.len() as u32);
    out_blocks.push(ExpandedBlock {
        id,
        label: label.into(),
        ops: Vec::new(),
        terminator: ExpandedTerminator::Return,
    });
    id
}
