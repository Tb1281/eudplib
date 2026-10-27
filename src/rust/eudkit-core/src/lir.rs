use crate::error::{EudkitError, Result};
use crate::hir::{
    epd, BlockId, ByteBufId, Comparison, HirBinOp, HirExpr, HirModule, HirStmt, HirTerminator,
    HirUnaryOp, HirValue, Modifier, PlayerId, StockAction, StockCondition, VarId,
    END_OF_TRIGGERS_SENTINEL,
};
use crate::map_context::MapContext;
use std::collections::{HashMap, HashSet};
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

    pub fn vproc_count(&self) -> usize {
        self.objects
            .iter()
            .map(|o| match o {
                LirObject::VProcBuffer(vb) => vb.slots.len(),
                _ => 0,
            })
            .sum()
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

struct ScratchAllocator {
    pool: Vec<VarId>,
    in_use: Vec<bool>,
}

impl ScratchAllocator {
    fn new() -> Self {
        Self {
            pool: Vec::new(),
            in_use: Vec::new(),
        }
    }

    fn acquire(&mut self, hir: &mut HirModule) -> VarId {
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

    fn release(&mut self, var: VarId) {
        for (i, &v) in self.pool.iter().enumerate() {
            if v == var {
                self.in_use[i] = false;
                return;
            }
        }
    }
}

#[derive(Debug, Clone)]
enum ExpandedOp {
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
}

fn push_var_const(out: &mut Vec<ExpandedOp>, var: VarId, modifier: Modifier, value: i32, mask: Option<u32>) {
    out.push(ExpandedOp::LeafAction(StockAction::SetVarConst {
        var,
        modifier,
        value,
        mask,
    }));
}

fn push_inplace_bitnot(out: &mut Vec<ExpandedOp>, var: VarId) {
    push_var_const(out, var, Modifier::Add, -1, Some(0x5555_5555));
    push_var_const(out, var, Modifier::Add, -1, Some(0xAAAA_AAAA));
}

fn push_inplace_neg(out: &mut Vec<ExpandedOp>, var: VarId) {
    push_inplace_bitnot(out, var);
    push_var_const(out, var, Modifier::Add, 1, None);
}

fn push_inplace_const_shl(out: &mut Vec<ExpandedOp>, var: VarId, n: u32) {
    if n == 0 {
        return;
    }
    if n >= 32 {
        push_var_const(out, var, Modifier::SetTo, 0, None);
        return;
    }
    let mask: u64 = (1u64 << (n + 1)) - 1;
    let half_mask: u64 = mask >> 1;
    for t in (0..(32 - n)).rev() {
        let clear_mask = ((half_mask << (t + 1)) & 0xFFFF_FFFF) as u32;
        let add_val = ((half_mask << t) & 0xFFFF_FFFF) as i32;
        let add_mask = ((mask << t) & 0xFFFF_FFFF) as u32;
        push_var_const(out, var, Modifier::SetTo, 0, Some(clear_mask));
        push_var_const(out, var, Modifier::Add, add_val, Some(add_mask));
    }
    push_var_const(out, var, Modifier::SetTo, 0, Some(half_mask as u32));
}

fn push_inplace_const_shr(out: &mut Vec<ExpandedOp>, var: VarId, n: u32) {
    if n == 0 {
        return;
    }
    if n >= 32 {
        push_var_const(out, var, Modifier::SetTo, 0, None);
        return;
    }
    let mask: u64 = (1u64 << (n + 1)) - 1;
    let half_mask: u64 = mask >> 1;
    push_var_const(out, var, Modifier::SetTo, 0, Some(half_mask as u32));
    for t in 0..(32 - n) {
        let sub_val = ((half_mask << t) & 0xFFFF_FFFF) as i32;
        let sub_mask = ((mask << t) & 0xFFFF_FFFF) as u32;
        push_var_const(out, var, Modifier::Subtract, sub_val, Some(sub_mask));
    }
}

fn expr_references_var(expr: &HirExpr, var: VarId) -> bool {
    match expr {
        HirExpr::Const(_) => false,
        HirExpr::Var(v) => *v == var,
        HirExpr::UnaryOp { operand, .. } => expr_references_var(operand, var),
        HirExpr::BinOp { lhs, rhs, .. } => {
            expr_references_var(lhs, var) || expr_references_var(rhs, var)
        }
    }
}

fn lower_expr_into(
    expr: &HirExpr,
    dst: VarId,
    hir: &mut HirModule,
    scratch: &mut ScratchAllocator,
    out: &mut Vec<ExpandedOp>,
) {
    let folded = expr.fold_constants();
    match &folded {
        HirExpr::Const(c) => {
            push_var_const(out, dst, Modifier::SetTo, *c, None);
        }
        HirExpr::Var(src) => {
            if *src != dst {
                out.push(ExpandedOp::VProcCopyVar {
                    src: *src,
                    dst,
                    modifier: Modifier::SetTo,
                    mask: None,
                });
            }
        }
        HirExpr::UnaryOp { op, operand } => {
            if let HirExpr::Var(src) = operand.as_ref() {
                if *src != dst {
                    push_var_const(out, dst, Modifier::SetTo, -1, None);
                    out.push(ExpandedOp::VProcCopyVar {
                        src: *src,
                        dst,
                        modifier: Modifier::Subtract,
                        mask: None,
                    });
                    if *op == HirUnaryOp::Neg {
                        push_var_const(out, dst, Modifier::Add, 1, None);
                    }
                    return;
                }
            }
            lower_expr_into(operand, dst, hir, scratch, out);
            match op {
                HirUnaryOp::BitNot => push_inplace_bitnot(out, dst),
                HirUnaryOp::Neg => push_inplace_neg(out, dst),
            }
        }
        HirExpr::BinOp { op, lhs, rhs } => {
            match (op, lhs.as_ref(), rhs.as_ref()) {
                // Fast path: var + const or const + var
                (HirBinOp::Add, HirExpr::Var(src), HirExpr::Const(c))
                | (HirBinOp::Add, HirExpr::Const(c), HirExpr::Var(src)) => {
                    if *src == dst {
                        push_var_const(out, dst, Modifier::Add, *c, None);
                    } else {
                        push_var_const(out, dst, Modifier::SetTo, *c, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *src,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    }
                    return;
                }
                // Fast path: var - const -> var + (-const)
                (HirBinOp::Sub, HirExpr::Var(src), HirExpr::Const(c)) => {
                    let neg_c = (*c as u32).wrapping_neg() as i32;
                    if *src == dst {
                        push_var_const(out, dst, Modifier::Add, neg_c, None);
                    } else {
                        push_var_const(out, dst, Modifier::SetTo, neg_c, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *src,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    }
                    return;
                }
                // Fast path: const - var -> (~0 - var) + (const + 1)
                (HirBinOp::Sub, HirExpr::Const(c), HirExpr::Var(src)) => {
                    if *src != dst {
                        push_var_const(out, dst, Modifier::SetTo, -1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *src,
                            dst,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                        push_var_const(
                            out,
                            dst,
                            Modifier::Add,
                            (*c as u32).wrapping_add(1) as i32,
                            None,
                        );
                    } else {
                        push_inplace_bitnot(out, dst);
                        push_var_const(
                            out,
                            dst,
                            Modifier::Add,
                            (*c as u32).wrapping_add(1) as i32,
                            None,
                        );
                    }
                    return;
                }
                // Fast path: var1 + var2
                (HirBinOp::Add, HirExpr::Var(v1), HirExpr::Var(v2)) => {
                    if *v1 == dst {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *v2,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    } else if *v2 == dst {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *v1,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    } else {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *v1,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *v2,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    }
                    return;
                }
                // Fast path: var1 - var2 (32-bit unsigned two's-complement wrap: (~0 - var2) + 1 + var1)
                (HirBinOp::Sub, HirExpr::Var(v1), HirExpr::Var(v2)) => {
                    if *v1 != dst && *v2 != dst {
                        push_var_const(out, dst, Modifier::SetTo, -1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *v2,
                            dst,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                        push_var_const(out, dst, Modifier::Add, 1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *v1,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                        return;
                    }
                }
                // Fast path: expr & const / const & expr
                (HirBinOp::BitAnd, sub, HirExpr::Const(mask))
                | (HirBinOp::BitAnd, HirExpr::Const(mask), sub) => {
                    lower_expr_into(sub, dst, hir, scratch, out);
                    let inv_mask = !(*mask as u32);
                    push_var_const(out, dst, Modifier::SetTo, 0, Some(inv_mask));
                    return;
                }
                // Fast path: expr | const / const | expr
                (HirBinOp::BitOr, sub, HirExpr::Const(mask))
                | (HirBinOp::BitOr, HirExpr::Const(mask), sub) => {
                    lower_expr_into(sub, dst, hir, scratch, out);
                    push_var_const(out, dst, Modifier::SetTo, -1, Some(*mask as u32));
                    return;
                }
                // Fast path: expr ^ const / const ^ expr
                (HirBinOp::BitXor, sub, HirExpr::Const(mask))
                | (HirBinOp::BitXor, HirExpr::Const(mask), sub) => {
                    lower_expr_into(sub, dst, hir, scratch, out);
                    push_var_const(out, dst, Modifier::Add, *mask, Some(0x5555_5555));
                    push_var_const(out, dst, Modifier::Add, *mask, Some(0xAAAA_AAAA));
                    return;
                }
                // Fast path: expr << const
                (HirBinOp::Shl, sub, HirExpr::Const(c)) => {
                    let shift = *c as u32;
                    lower_expr_into(sub, dst, hir, scratch, out);
                    push_inplace_const_shl(out, dst, shift);
                    return;
                }
                // Fast path: expr >> const
                (HirBinOp::Shr, sub, HirExpr::Const(c)) => {
                    let shift = *c as u32;
                    lower_expr_into(sub, dst, hir, scratch, out);
                    push_inplace_const_shr(out, dst, shift);
                    return;
                }
                // Fast path: expr * const / const * expr
                (HirBinOp::Mul, sub, HirExpr::Const(c))
                | (HirBinOp::Mul, HirExpr::Const(c), sub) => {
                    let num = *c as u32;
                    let src_var = if let HirExpr::Var(v) = sub {
                        if *v != dst {
                            None
                        } else {
                            Some(scratch.acquire(hir))
                        }
                    } else {
                        Some(scratch.acquire(hir))
                    };
                    let read_var = if let Some(sv) = src_var {
                        lower_expr_into(sub, sv, hir, scratch, out);
                        sv
                    } else if let HirExpr::Var(v) = sub {
                        *v
                    } else {
                        unreachable!()
                    };
                    push_var_const(out, dst, Modifier::SetTo, 0, None);
                    for i in 0..32u32 {
                        let bit = 1u32 << i;
                        let term = bit.wrapping_mul(num);
                        if term == 0 {
                            continue;
                        }
                        out.push(ExpandedOp::Trigger {
                            label: Some(format!("mul_const_bit_{i}")),
                            conditions: vec![StockCondition::VarCmp {
                                var: read_var,
                                comparison: Comparison::AtLeast,
                                value: 1,
                                mask: Some(bit),
                            }],
                            actions: vec![StockAction::SetVarConst {
                                var: dst,
                                modifier: Modifier::Add,
                                value: term as i32,
                                mask: None,
                            }],
                            preserved: true,
                        });
                    }
                    if let Some(sv) = src_var {
                        scratch.release(sv);
                    }
                    return;
                }
                // Fast path: expr // const or expr % const
                (HirBinOp::Div | HirBinOp::Rem, sub, HirExpr::Const(c)) => {
                    let num = *c as u32;
                    if *op == HirBinOp::Rem {
                        lower_expr_into(sub, dst, hir, scratch, out);
                        for i in (0..32u32).rev() {
                            let prod = (1u64 << i) * (num as u64);
                            if prod >= (1u64 << 32) {
                                continue;
                            }
                            let prod_u32 = prod as u32;
                            out.push(ExpandedOp::Trigger {
                                label: Some(format!("rem_const_bit_{i}")),
                                conditions: vec![StockCondition::VarCmp {
                                    var: dst,
                                    comparison: Comparison::AtLeast,
                                    value: prod_u32,
                                    mask: None,
                                }],
                                actions: vec![StockAction::SetVarConst {
                                    var: dst,
                                    modifier: Modifier::Subtract,
                                    value: prod_u32 as i32,
                                    mask: None,
                                }],
                                preserved: true,
                            });
                        }
                        return;
                    }
                    let rem_var = scratch.acquire(hir);
                    lower_expr_into(sub, rem_var, hir, scratch, out);
                    push_var_const(out, dst, Modifier::SetTo, 0, None);
                    for i in (0..32u32).rev() {
                        let prod = (1u64 << i) * (num as u64);
                        if prod >= (1u64 << 32) {
                            continue;
                        }
                        let prod_u32 = prod as u32;
                        let acts = vec![
                            StockAction::SetVarConst {
                                var: rem_var,
                                modifier: Modifier::Subtract,
                                value: prod_u32 as i32,
                                mask: None,
                            },
                            StockAction::SetVarConst {
                                var: dst,
                                modifier: Modifier::Add,
                                value: (1u32 << i) as i32,
                                mask: None,
                            },
                        ];
                        out.push(ExpandedOp::Trigger {
                            label: Some(format!("div_const_bit_{i}")),
                            conditions: vec![StockCondition::VarCmp {
                                var: rem_var,
                                comparison: Comparison::AtLeast,
                                value: prod_u32,
                                mask: None,
                            }],
                            actions: acts,
                            preserved: true,
                        });
                    }
                    scratch.release(rem_var);
                    return;
                }
                _ => {}
            }

            // General binary op: if `dst` is not referenced by `rhs`, we can evaluate `lhs`
            // directly into `dst` (for associative/commutative/in-place ops like Add, Sub, SubSat,
            // BitAnd, BitOr, BitXor), saving an entire scratch VProc slot!
            if matches!(
                op,
                HirBinOp::Add
                    | HirBinOp::Sub
                    | HirBinOp::SubSat
                    | HirBinOp::BitAnd
                    | HirBinOp::BitOr
                    | HirBinOp::BitXor
            ) && !expr_references_var(rhs, dst)
            {
                lower_expr_into(lhs, dst, hir, scratch, out);
                if *op == HirBinOp::Sub {
                    if let HirExpr::Var(rv) = rhs.as_ref() {
                        let inv_tmp = scratch.acquire(hir);
                        push_var_const(out, inv_tmp, Modifier::SetTo, -1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: *rv,
                            dst: inv_tmp,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                        push_var_const(out, dst, Modifier::Add, 1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: inv_tmp,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                        scratch.release(inv_tmp);
                    } else {
                        let tmp = scratch.acquire(hir);
                        lower_expr_into(rhs, tmp, hir, scratch, out);
                        push_inplace_bitnot(out, tmp);
                        push_var_const(out, dst, Modifier::Add, 1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: tmp,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                        scratch.release(tmp);
                    }
                    return;
                }
                if *op == HirBinOp::SubSat {
                    if let HirExpr::Const(c) = rhs.as_ref() {
                        push_var_const(out, dst, Modifier::Subtract, *c, None);
                        return;
                    }
                }
                if *op == HirBinOp::Add {
                    match rhs.as_ref() {
                        HirExpr::BinOp {
                            op: HirBinOp::Mul,
                            lhs: m_l,
                            rhs: m_r,
                        } if matches!(
                            (m_l.as_ref(), m_r.as_ref()),
                            (_, HirExpr::Const(_)) | (HirExpr::Const(_), _)
                        ) =>
                        {
                            let (sub, c) = match (m_l.as_ref(), m_r.as_ref()) {
                                (s, HirExpr::Const(c)) | (HirExpr::Const(c), s) => (s, *c as u32),
                                _ => unreachable!(),
                            };
                            let (read_var, owned) = match sub {
                                HirExpr::Var(v) if *v != dst => (*v, false),
                                _ => {
                                    let sv = scratch.acquire(hir);
                                    lower_expr_into(sub, sv, hir, scratch, out);
                                    (sv, true)
                                }
                            };
                            for i in 0..32u32 {
                                let bit = 1u32 << i;
                                let term = bit.wrapping_mul(c);
                                if term == 0 {
                                    continue;
                                }
                                out.push(ExpandedOp::Trigger {
                                    label: Some(format!("add_mul_const_bit_{i}")),
                                    conditions: vec![StockCondition::VarCmp {
                                        var: read_var,
                                        comparison: Comparison::AtLeast,
                                        value: 1,
                                        mask: Some(bit),
                                    }],
                                    actions: vec![StockAction::SetVarConst {
                                        var: dst,
                                        modifier: Modifier::Add,
                                        value: term as i32,
                                        mask: None,
                                    }],
                                    preserved: true,
                                });
                            }
                            if owned {
                                scratch.release(read_var);
                            }
                            return;
                        }
                        HirExpr::BinOp {
                            op: HirBinOp::Div,
                            lhs: d_sub,
                            rhs: d_rhs,
                        } if matches!(d_rhs.as_ref(), HirExpr::Const(_)) => {
                            let num = match d_rhs.as_ref() {
                                HirExpr::Const(c) => *c as u32,
                                _ => unreachable!(),
                            };
                            let rem_var = scratch.acquire(hir);
                            lower_expr_into(d_sub, rem_var, hir, scratch, out);
                            for i in (0..32u32).rev() {
                                let prod = (1u64 << i) * (num as u64);
                                if prod >= (1u64 << 32) {
                                    continue;
                                }
                                let prod_u32 = prod as u32;
                                out.push(ExpandedOp::Trigger {
                                    label: Some(format!("add_div_const_bit_{i}")),
                                    conditions: vec![StockCondition::VarCmp {
                                        var: rem_var,
                                        comparison: Comparison::AtLeast,
                                        value: prod_u32,
                                        mask: None,
                                    }],
                                    actions: vec![
                                        StockAction::SetVarConst {
                                            var: rem_var,
                                            modifier: Modifier::Subtract,
                                            value: prod_u32 as i32,
                                            mask: None,
                                        },
                                        StockAction::SetVarConst {
                                            var: dst,
                                            modifier: Modifier::Add,
                                            value: (1u32 << i) as i32,
                                            mask: None,
                                        },
                                    ],
                                    preserved: true,
                                });
                            }
                            scratch.release(rem_var);
                            return;
                        }
                        _ => {}
                    }
                }
                let (rhs_var, rhs_owned) = match rhs.as_ref() {
                    HirExpr::Var(v) => (*v, false),
                    _ => {
                        let tmp = scratch.acquire(hir);
                        lower_expr_into(rhs, tmp, hir, scratch, out);
                        (tmp, true)
                    }
                };
                match op {
                    HirBinOp::Add => out.push(ExpandedOp::VProcCopyVar {
                        src: rhs_var,
                        dst,
                        modifier: Modifier::Add,
                        mask: None,
                    }),
                    HirBinOp::SubSat => out.push(ExpandedOp::VProcCopyVar {
                        src: rhs_var,
                        dst,
                        modifier: Modifier::Subtract,
                        mask: None,
                    }),
                    HirBinOp::BitAnd => out.push(ExpandedOp::VProcBitAndVar { src: rhs_var, dst }),
                    HirBinOp::BitOr => out.push(ExpandedOp::VProcBitOrVar { src: rhs_var, dst }),
                    HirBinOp::BitXor => out.push(ExpandedOp::VProcBitXorVar { src: rhs_var, dst }),
                    _ => unreachable!(),
                }
                if rhs_owned {
                    scratch.release(rhs_var);
                }
                return;
            }

            let (lhs_var, lhs_owned) = match lhs.as_ref() {
                HirExpr::Var(v) => (*v, false),
                _ => {
                    let tmp = scratch.acquire(hir);
                    lower_expr_into(lhs, tmp, hir, scratch, out);
                    (tmp, true)
                }
            };
            let (rhs_var, rhs_owned) = match rhs.as_ref() {
                HirExpr::Var(v) => (*v, false),
                _ => {
                    let tmp = scratch.acquire(hir);
                    lower_expr_into(rhs, tmp, hir, scratch, out);
                    (tmp, true)
                }
            };

            match op {
                HirBinOp::Add => {
                    if lhs_var == dst {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    } else if rhs_var == dst {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    } else {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    }
                }
                HirBinOp::Sub => {
                    if lhs_var != dst && rhs_var != dst {
                        push_var_const(out, dst, Modifier::SetTo, -1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                        push_var_const(out, dst, Modifier::Add, 1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    } else if lhs_var == dst && rhs_var != dst {
                        let inv_tmp = scratch.acquire(hir);
                        push_var_const(out, inv_tmp, Modifier::SetTo, -1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst: inv_tmp,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                        push_var_const(out, dst, Modifier::Add, 1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: inv_tmp,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                        scratch.release(inv_tmp);
                    } else if rhs_var == dst && lhs_var != dst {
                        push_inplace_bitnot(out, dst);
                        push_var_const(out, dst, Modifier::Add, 1, None);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::Add,
                            mask: None,
                        });
                    } else {
                        push_var_const(out, dst, Modifier::SetTo, 0, None);
                    }
                }
                HirBinOp::SubSat => {
                    if rhs_var != dst {
                        if lhs_var != dst {
                            out.push(ExpandedOp::VProcCopyVar {
                                src: lhs_var,
                                dst,
                                modifier: Modifier::SetTo,
                                mask: None,
                            });
                        }
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                    } else if lhs_var != dst {
                        let rhs_tmp = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst: rhs_tmp,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_tmp,
                            dst,
                            modifier: Modifier::Subtract,
                            mask: None,
                        });
                        scratch.release(rhs_tmp);
                    } else {
                        push_var_const(out, dst, Modifier::SetTo, 0, None);
                    }
                }
                HirBinOp::BitAnd => {
                    if lhs_var == dst {
                        out.push(ExpandedOp::VProcBitAndVar { src: rhs_var, dst });
                    } else if rhs_var == dst {
                        out.push(ExpandedOp::VProcBitAndVar { src: lhs_var, dst });
                    } else {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcBitAndVar { src: rhs_var, dst });
                    }
                }
                HirBinOp::BitOr => {
                    if lhs_var == dst {
                        out.push(ExpandedOp::VProcBitOrVar { src: rhs_var, dst });
                    } else if rhs_var == dst {
                        out.push(ExpandedOp::VProcBitOrVar { src: lhs_var, dst });
                    } else {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcBitOrVar { src: rhs_var, dst });
                    }
                }
                HirBinOp::BitXor => {
                    if lhs_var == dst {
                        out.push(ExpandedOp::VProcBitXorVar { src: rhs_var, dst });
                    } else if rhs_var == dst {
                        out.push(ExpandedOp::VProcBitXorVar { src: lhs_var, dst });
                    } else {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        out.push(ExpandedOp::VProcBitXorVar { src: rhs_var, dst });
                    }
                }
                HirBinOp::Shl => {
                    let shift_tmp = if rhs_owned {
                        rhs_var
                    } else {
                        let st = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst: st,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        st
                    };
                    if lhs_var != dst {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                    }
                    out.push(ExpandedOp::DynamicShl {
                        dst,
                        shift: shift_tmp,
                    });
                    if !rhs_owned {
                        scratch.release(shift_tmp);
                    }
                }
                HirBinOp::Shr => {
                    let div_q_scratch = if lhs_owned {
                        lhs_var
                    } else {
                        let qt = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst: qt,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        qt
                    };
                    let pow2_scratch = scratch.acquire(hir);
                    let div_x_scratch = scratch.acquire(hir);
                    out.push(ExpandedOp::DynamicShr {
                        dst,
                        shift: rhs_var,
                        pow2_scratch,
                        div_x_scratch,
                        div_q_scratch,
                    });
                    scratch.release(div_x_scratch);
                    scratch.release(pow2_scratch);
                    if !lhs_owned {
                        scratch.release(div_q_scratch);
                    }
                }
                HirBinOp::Mul => {
                    let rhs_mut = if rhs_owned {
                        rhs_var
                    } else {
                        let rt = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: rhs_var,
                            dst: rt,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        rt
                    };
                    let lhs_safe = if lhs_var == dst {
                        let lt = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst: lt,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        Some(lt)
                    } else {
                        None
                    };
                    out.push(ExpandedOp::DynamicMul {
                        lhs: lhs_safe.unwrap_or(lhs_var),
                        rhs: rhs_mut,
                        dst,
                    });
                    if let Some(lt) = lhs_safe {
                        scratch.release(lt);
                    }
                    if !rhs_owned {
                        scratch.release(rhs_mut);
                    }
                }
                HirBinOp::Div => {
                    let div_tmp = if lhs_owned && lhs_var != dst {
                        lhs_var
                    } else {
                        let dt = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst: dt,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        dt
                    };
                    let div_x_scratch = scratch.acquire(hir);
                    out.push(ExpandedOp::DynamicDivRem {
                        dividend: div_tmp,
                        divisor: rhs_var,
                        quot_dst: dst,
                        div_x_scratch,
                    });
                    scratch.release(div_x_scratch);
                    if !(lhs_owned && lhs_var != dst) {
                        scratch.release(div_tmp);
                    }
                }
                HirBinOp::Rem => {
                    let div_tmp = if lhs_owned {
                        lhs_var
                    } else {
                        let dt = scratch.acquire(hir);
                        out.push(ExpandedOp::VProcCopyVar {
                            src: lhs_var,
                            dst: dt,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                        dt
                    };
                    let div_x_scratch = scratch.acquire(hir);
                    let quot_scratch = scratch.acquire(hir);
                    out.push(ExpandedOp::DynamicDivRem {
                        dividend: div_tmp,
                        divisor: rhs_var,
                        quot_dst: quot_scratch,
                        div_x_scratch,
                    });
                    if div_tmp != dst {
                        out.push(ExpandedOp::VProcCopyVar {
                            src: div_tmp,
                            dst,
                            modifier: Modifier::SetTo,
                            mask: None,
                        });
                    }
                    scratch.release(quot_scratch);
                    scratch.release(div_x_scratch);
                    if !lhs_owned {
                        scratch.release(div_tmp);
                    }
                }
            }

            if rhs_owned {
                scratch.release(rhs_var);
            }
            if lhs_owned {
                scratch.release(lhs_var);
            }
        }
    }
}

fn eval_expr_to_var_or_const(
    expr: &HirExpr,
    hir: &mut HirModule,
    scratch: &mut ScratchAllocator,
    out: &mut Vec<ExpandedOp>,
) -> (Option<VarId>, Option<i32>, bool) {
    let folded = expr.fold_constants();
    match folded {
        HirExpr::Const(c) => (None, Some(c), false),
        HirExpr::Var(v) => (Some(v), None, false),
        other => {
            let tmp = scratch.acquire(hir);
            lower_expr_into(&other, tmp, hir, scratch, out);
            (Some(tmp), None, true)
        }
    }
}

fn prepare_conditions(
    conditions: &[StockCondition],
    hir: &mut HirModule,
    scratch: &mut ScratchAllocator,
    pre_ops: &mut Vec<ExpandedOp>,
) -> Vec<StockCondition> {
    let mut lowered_conds = Vec::with_capacity(conditions.len());
    let mut patch_ops: Vec<ExpandedOp> = Vec::new();
    let mut scratch_to_release: Vec<VarId> = Vec::new();

    for cond in conditions {
        let cond_idx = lowered_conds.len() as u8;
        match cond {
            StockCondition::ExprCmp {
                lhs,
                comparison,
                rhs,
            } => {
                let lf = lhs.fold_constants();
                let rf = rhs.fold_constants();
                match (&lf, &rf) {
                    (HirExpr::Var(v), HirExpr::Const(c)) => {
                        lowered_conds.push(StockCondition::VarCmp {
                            var: *v,
                            comparison: *comparison,
                            value: *c as u32,
                            mask: None,
                        });
                    }
                    (HirExpr::Const(c), HirExpr::Var(v)) => {
                        let swapped = match comparison {
                            Comparison::Exactly => Comparison::Exactly,
                            Comparison::AtLeast => Comparison::AtMost,
                            Comparison::AtMost => Comparison::AtLeast,
                        };
                        lowered_conds.push(StockCondition::VarCmp {
                            var: *v,
                            comparison: swapped,
                            value: *c as u32,
                            mask: None,
                        });
                    }
                    (
                        HirExpr::BinOp {
                            op: HirBinOp::BitAnd,
                            lhs: and_l,
                            rhs: and_r,
                        },
                        HirExpr::Const(c),
                    ) if matches!(
                        (and_l.as_ref(), and_r.as_ref()),
                        (HirExpr::Var(_), HirExpr::Const(_)) | (HirExpr::Const(_), HirExpr::Var(_))
                    ) =>
                    {
                        let (v, m) = match (and_l.as_ref(), and_r.as_ref()) {
                            (HirExpr::Var(v), HirExpr::Const(m))
                            | (HirExpr::Const(m), HirExpr::Var(v)) => (*v, *m as u32),
                            _ => unreachable!(),
                        };
                        lowered_conds.push(StockCondition::VarCmp {
                            var: v,
                            comparison: *comparison,
                            value: *c as u32,
                            mask: Some(m),
                        });
                    }
                    _ => {
                        let (l_var, l_const, l_owned) =
                            eval_expr_to_var_or_const(&lf, hir, scratch, pre_ops);
                        let (r_var, r_const, r_owned) =
                            eval_expr_to_var_or_const(&rf, hir, scratch, pre_ops);
                        if l_owned {
                            scratch_to_release.push(l_var.unwrap());
                        }
                        if r_owned {
                            scratch_to_release.push(r_var.unwrap());
                        }
                        match (l_var, l_const, r_var, r_const) {
                            (Some(lv), None, Some(rv), None) => {
                                lowered_conds.push(StockCondition::VarCmp {
                                    var: lv,
                                    comparison: *comparison,
                                    value: 0,
                                    mask: None,
                                });
                                patch_ops.push(ExpandedOp::PatchNextTriggerCond {
                                    src: rv,
                                    cond_idx,
                                    field: CondField::Amount,
                                    is_epd: false,
                                });
                            }
                            (Some(lv), None, None, Some(rc)) => {
                                lowered_conds.push(StockCondition::VarCmp {
                                    var: lv,
                                    comparison: *comparison,
                                    value: rc as u32,
                                    mask: None,
                                });
                            }
                            (None, Some(lc), Some(rv), None) => {
                                let swapped = match comparison {
                                    Comparison::Exactly => Comparison::Exactly,
                                    Comparison::AtLeast => Comparison::AtMost,
                                    Comparison::AtMost => Comparison::AtLeast,
                                };
                                lowered_conds.push(StockCondition::VarCmp {
                                    var: rv,
                                    comparison: swapped,
                                    value: lc as u32,
                                    mask: None,
                                });
                            }
                            (None, Some(lc), None, Some(rc)) => {
                                let lu = lc as u32;
                                let ru = rc as u32;
                                let ok = match comparison {
                                    Comparison::Exactly => lu == ru,
                                    Comparison::AtLeast => lu >= ru,
                                    Comparison::AtMost => lu <= ru,
                                };
                                lowered_conds.push(if ok {
                                    StockCondition::Always
                                } else {
                                    StockCondition::Never
                                });
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
            StockCondition::DeathsDynamic {
                player,
                comparison,
                amount,
                unit,
                mask,
            } => {
                let (p_var, p_const, p_owned) =
                    eval_expr_to_var_or_const(player, hir, scratch, pre_ops);
                let (a_var, a_const, a_owned) =
                    eval_expr_to_var_or_const(amount, hir, scratch, pre_ops);
                if p_owned {
                    scratch_to_release.push(p_var.unwrap());
                }
                if a_owned {
                    scratch_to_release.push(a_var.unwrap());
                }
                let p_static = PlayerId::Epd(p_const.unwrap_or(0));
                let a_static = a_const.unwrap_or(0) as u32;
                lowered_conds.push(match mask {
                    None => StockCondition::Deaths {
                        player: p_static,
                        comparison: *comparison,
                        amount: a_static,
                        unit: *unit,
                    },
                    Some(m) => StockCondition::DeathsX {
                        player: p_static,
                        comparison: *comparison,
                        amount: a_static,
                        unit: *unit,
                        mask: *m,
                    },
                });
                if let Some(pv) = p_var {
                    patch_ops.push(ExpandedOp::PatchNextTriggerCond {
                        src: pv,
                        cond_idx,
                        field: CondField::Player,
                        is_epd: false,
                    });
                }
                if let Some(av) = a_var {
                    patch_ops.push(ExpandedOp::PatchNextTriggerCond {
                        src: av,
                        cond_idx,
                        field: CondField::Amount,
                        is_epd: false,
                    });
                }
            }
            StockCondition::MemoryDynamic {
                address,
                comparison,
                value,
                mask,
            } => {
                let (v_var, v_const, v_owned) =
                    eval_expr_to_var_or_const(value, hir, scratch, pre_ops);
                if v_owned {
                    scratch_to_release.push(v_var.unwrap());
                }
                let v_static = v_const.unwrap_or(0) as u32;
                lowered_conds.push(match mask {
                    None => StockCondition::Memory {
                        address: *address,
                        comparison: *comparison,
                        value: v_static,
                    },
                    Some(m) => StockCondition::MemoryX {
                        address: *address,
                        comparison: *comparison,
                        value: v_static,
                        mask: *m,
                    },
                });
                if let Some(vv) = v_var {
                    patch_ops.push(ExpandedOp::PatchNextTriggerCond {
                        src: vv,
                        cond_idx,
                        field: CondField::Amount,
                        is_epd: false,
                    });
                }
            }
            other => lowered_conds.push(other.clone()),
        }
    }

    pre_ops.extend(patch_ops);
    for sv in scratch_to_release {
        scratch.release(sv);
    }
    lowered_conds
}

fn lower_action_into_ops(
    act: &StockAction,
    hir: &mut HirModule,
    scratch: &mut ScratchAllocator,
    out: &mut Vec<ExpandedOp>,
) {
    match act {
        StockAction::SetDeathsDynamic {
            player,
            modifier,
            value,
            unit,
            mask,
        } => {
            let (p_var, p_const, p_owned) = eval_expr_to_var_or_const(player, hir, scratch, out);
            let (v_var, v_const, v_owned) = eval_expr_to_var_or_const(value, hir, scratch, out);
            if p_var.is_none() && v_var.is_none() {
                let p_id = PlayerId::Epd(p_const.unwrap_or(0));
                let val = HirValue::Const(v_const.unwrap_or(0));
                out.push(ExpandedOp::LeafAction(match mask {
                    None => StockAction::SetDeaths {
                        player: p_id,
                        modifier: *modifier,
                        value: val,
                        unit: *unit,
                    },
                    Some(m) => StockAction::SetDeathsX {
                        player: p_id,
                        modifier: *modifier,
                        value: val,
                        unit: *unit,
                        mask: *m,
                    },
                }));
            } else {
                if let Some(pv) = p_var {
                    out.push(ExpandedOp::PatchNextTriggerAct {
                        src: pv,
                        act_idx: 0,
                        field: ActField::Player1,
                        is_epd: false,
                    });
                }
                if let Some(vv) = v_var {
                    out.push(ExpandedOp::PatchNextTriggerAct {
                        src: vv,
                        act_idx: 0,
                        field: ActField::Player2,
                        is_epd: false,
                    });
                }
                let p_id = PlayerId::Epd(p_const.unwrap_or(0));
                let val = HirValue::Const(v_const.unwrap_or(0));
                let static_act = match mask {
                    None => StockAction::SetDeaths {
                        player: p_id,
                        modifier: *modifier,
                        value: val,
                        unit: *unit,
                    },
                    Some(m) => StockAction::SetDeathsX {
                        player: p_id,
                        modifier: *modifier,
                        value: val,
                        unit: *unit,
                        mask: *m,
                    },
                };
                out.push(ExpandedOp::Trigger {
                    label: Some("dyn_set_deaths".to_string()),
                    conditions: Vec::new(),
                    actions: vec![static_act],
                    preserved: true,
                });
            }
            if p_owned {
                scratch.release(p_var.unwrap());
            }
            if v_owned {
                scratch.release(v_var.unwrap());
            }
        }
        StockAction::SetMemoryDynamic {
            address,
            modifier,
            value,
            mask,
        } => {
            let (v_var, v_const, v_owned) = eval_expr_to_var_or_const(value, hir, scratch, out);
            if let Some(vv) = v_var {
                out.push(ExpandedOp::PatchNextTriggerAct {
                    src: vv,
                    act_idx: 0,
                    field: ActField::Player2,
                    is_epd: false,
                });
                let static_act = match mask {
                    None => StockAction::SetMemory {
                        address: *address,
                        modifier: *modifier,
                        value: HirValue::Const(0),
                    },
                    Some(m) => StockAction::SetMemoryX {
                        address: *address,
                        modifier: *modifier,
                        value: HirValue::Const(0),
                        mask: *m,
                    },
                };
                out.push(ExpandedOp::Trigger {
                    label: Some("dyn_set_memory".to_string()),
                    conditions: Vec::new(),
                    actions: vec![static_act],
                    preserved: true,
                });
            } else {
                let c = v_const.unwrap_or(0);
                out.push(ExpandedOp::LeafAction(match mask {
                    None => StockAction::SetMemory {
                        address: *address,
                        modifier: *modifier,
                        value: HirValue::Const(c),
                    },
                    Some(m) => StockAction::SetMemoryX {
                        address: *address,
                        modifier: *modifier,
                        value: HirValue::Const(c),
                        mask: *m,
                    },
                }));
            }
            if v_owned {
                scratch.release(v_var.unwrap());
            }
        }
        StockAction::SetFieldDynamic {
            target,
            modifier,
            value,
            mask,
        } => {
            let (v_var, v_const, v_owned) = eval_expr_to_var_or_const(value, hir, scratch, out);
            if let Some(vv) = v_var {
                out.push(ExpandedOp::PatchNextTriggerAct {
                    src: vv,
                    act_idx: 0,
                    field: ActField::Player2,
                    is_epd: false,
                });
                let static_act = match mask {
                    None => StockAction::SetField {
                        target: *target,
                        modifier: *modifier,
                        value: HirValue::Const(0),
                    },
                    Some(m) => StockAction::SetDeathsX {
                        player: PlayerId::Epd(0),
                        modifier: *modifier,
                        value: HirValue::Const(0),
                        unit: 0,
                        mask: *m,
                    },
                };
                out.push(ExpandedOp::Trigger {
                    label: Some("dyn_set_field".to_string()),
                    conditions: Vec::new(),
                    actions: vec![static_act],
                    preserved: true,
                });
            } else {
                let c = v_const.unwrap_or(0);
                out.push(ExpandedOp::LeafAction(StockAction::SetField {
                    target: *target,
                    modifier: *modifier,
                    value: HirValue::Const(c),
                }));
            }
            if v_owned {
                scratch.release(v_var.unwrap());
            }
        }
        other => out.push(ExpandedOp::LeafAction(other.clone())),
    }
}

fn stmt_to_leaf_action(stmt: &HirStmt) -> Option<StockAction> {
    match stmt {
        HirStmt::Action(a)
            if !matches!(
                a,
                StockAction::SetDeathsDynamic { .. }
                    | StockAction::SetMemoryDynamic { .. }
                    | StockAction::SetFieldDynamic { .. }
            ) =>
        {
            Some(a.clone())
        }
        HirStmt::VarAssignConst { var, value } => Some(StockAction::SetVarConst {
            var: *var,
            modifier: Modifier::SetTo,
            value: *value,
            mask: None,
        }),
        HirStmt::VarAddConst { var, value } => Some(StockAction::SetVarConst {
            var: *var,
            modifier: Modifier::Add,
            value: *value,
            mask: None,
        }),
        HirStmt::VarSubConst { var, value } => Some(StockAction::SetVarConst {
            var: *var,
            modifier: Modifier::Subtract,
            value: *value,
            mask: None,
        }),
        HirStmt::VarSetMaskConst {
            var,
            modifier,
            value,
            mask,
        } => Some(StockAction::SetVarConst {
            var: *var,
            modifier: *modifier,
            value: *value,
            mask: Some(*mask),
        }),
        HirStmt::VarAssignExpr { var, expr } => {
            if let HirExpr::Const(c) = expr.fold_constants() {
                Some(StockAction::SetVarConst {
                    var: *var,
                    modifier: Modifier::SetTo,
                    value: c,
                    mask: None,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_leaf_actions(stmts: &[HirStmt]) -> Option<Vec<StockAction>> {
    let mut actions = Vec::with_capacity(stmts.len());
    for s in stmts {
        actions.push(stmt_to_leaf_action(s)?);
    }
    Some(actions)
}

fn actions_cannot_mutate_condition(actions: &[StockAction], cond: &StockCondition) -> bool {
    actions.iter().all(|act| match act {
        StockAction::DisplayText { .. }
        | StockAction::DisplayTextId { .. }
        | StockAction::Comment { .. }
        | StockAction::PreserveTrigger
        | StockAction::Victory
        | StockAction::Defeat => true,
        StockAction::CreateUnit { .. } => !matches!(
            cond,
            StockCondition::Command { .. } | StockCondition::Bring { .. }
        ),
        StockAction::SetResources { .. } => !matches!(cond, StockCondition::Accumulate { .. }),
        StockAction::SetVarConst { var: act_var, .. } => match cond {
            StockCondition::VarCmp { var: cond_var, .. } => act_var != cond_var,
            _ => true,
        },
        StockAction::SetDeaths { .. }
        | StockAction::SetDeathsX { .. }
        | StockAction::SetDeathsDynamic { .. }
        | StockAction::SetMemory { .. }
        | StockAction::SetMemoryX { .. }
        | StockAction::SetMemoryDynamic { .. }
        | StockAction::SetField { .. }
        | StockAction::SetFieldDynamic { .. } => false,
    })
}

#[derive(Debug, Clone)]
enum ExpandedTerminator {
    Jump(BlockId),
    Branch {
        conditions: Vec<StockCondition>,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return,
    FunctionReturn(crate::hir::FuncId),
}

#[derive(Debug, Clone)]
struct ExpandedBlock {
    id: BlockId,
    label: String,
    ops: Vec<ExpandedOp>,
    terminator: ExpandedTerminator,
}

#[derive(Debug, Clone)]
struct ExpandedCfg {
    blocks: Vec<ExpandedBlock>,
    fn_entry_block: HashMap<crate::hir::FuncId, BlockId>,
    fp_trampoline_block: HashMap<crate::hir::FuncPtrId, BlockId>,
}

fn alloc_block(out_blocks: &mut Vec<ExpandedBlock>, label: impl Into<String>) -> BlockId {
    let id = BlockId(out_blocks.len() as u32);
    out_blocks.push(ExpandedBlock {
        id,
        label: label.into(),
        ops: Vec::new(),
        terminator: ExpandedTerminator::Return,
    });
    id
}

fn lower_and_expand_cfg(
    hir: &mut HirModule,
    main_fn_id: crate::hir::FuncId,
    scratch: &mut ScratchAllocator,
) -> Result<ExpandedCfg> {
    fn lower_stmts(
        stmts: &[HirStmt],
        mut cur_block: BlockId,
        out_blocks: &mut Vec<ExpandedBlock>,
        hir: &mut HirModule,
        scratch: &mut ScratchAllocator,
        loop_ctx: Option<(BlockId, BlockId)>,
        current_fn_id: crate::hir::FuncId,
        fn_entry_block: &HashMap<crate::hir::FuncId, BlockId>,
        fp_trampoline_block: &HashMap<crate::hir::FuncPtrId, BlockId>,
    ) -> Result<Option<BlockId>> {
        for stmt in stmts {
            if let Some(act) = stmt_to_leaf_action(stmt) {
                out_blocks[cur_block.0 as usize]
                    .ops
                    .push(ExpandedOp::LeafAction(act));
                continue;
            }
            match stmt {
                HirStmt::Trigger {
                    label,
                    conditions,
                    actions,
                    preserved,
                } => {
                    let mut pre_ops = Vec::new();
                    let lowered_conds = prepare_conditions(conditions, hir, scratch, &mut pre_ops);
                    out_blocks[cur_block.0 as usize].ops.extend(pre_ops);
                    out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::Trigger {
                        label: label.clone(),
                        conditions: lowered_conds,
                        actions: actions.clone(),
                        preserved: *preserved,
                    });
                }
                HirStmt::Action(act) => {
                    lower_action_into_ops(
                        act,
                        hir,
                        scratch,
                        &mut out_blocks[cur_block.0 as usize].ops,
                    );
                }
                HirStmt::VarAssignExpr { var, expr } => {
                    lower_expr_into(
                        expr,
                        *var,
                        hir,
                        scratch,
                        &mut out_blocks[cur_block.0 as usize].ops,
                    );
                }
                HirStmt::VarAssignConst { .. }
                | HirStmt::VarAddConst { .. }
                | HirStmt::VarSubConst { .. }
                | HirStmt::VarSetMaskConst { .. } => unreachable!(),
                HirStmt::Call { callee, args, rets } => {
                    let callee_params = hir.functions[callee.0 as usize].params.clone();
                    for (i, arg) in args.iter().enumerate() {
                        if i < callee_params.len() {
                            lower_expr_into(arg, callee_params[i], hir, scratch, &mut out_blocks[cur_block.0 as usize].ops);
                        }
                    }
                    let return_block = alloc_block(out_blocks, format!("{}_call_ret", out_blocks[cur_block.0 as usize].label));
                    out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::CallSetup {
                        callee: *callee,
                        rets: rets.clone(),
                        return_block,
                    });
                    let callee_entry = fn_entry_block[callee];
                    out_blocks[cur_block.0 as usize].terminator = ExpandedTerminator::Jump(callee_entry);
                    cur_block = return_block;
                }
                HirStmt::CallIndirect { ptr, args, rets } => {
                    let fp = &hir.func_ptrs[ptr.0 as usize];
                    if let Some(target_fn) = fp.initial_target {
                        let target_params = hir.functions[target_fn.0 as usize].params.clone();
                        for (i, arg) in args.iter().enumerate() {
                            if i < target_params.len() {
                                lower_expr_into(arg, target_params[i], hir, scratch, &mut out_blocks[cur_block.0 as usize].ops);
                            }
                        }
                    }
                    let return_block = alloc_block(out_blocks, format!("{}_icall_ret", out_blocks[cur_block.0 as usize].label));
                    out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::CallIndirectSetup {
                        ptr: *ptr,
                        rets: rets.clone(),
                        return_block,
                    });
                    let tramp_entry = fp_trampoline_block[ptr];
                    out_blocks[cur_block.0 as usize].terminator = ExpandedTerminator::Jump(tramp_entry);
                    cur_block = return_block;
                }
                HirStmt::FuncPtrAssign { ptr, target } => {
                    out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::FuncPtrAssignTarget {
                        ptr: *ptr,
                        target: *target,
                    });
                }
                HirStmt::ReturnValues(exprs) => {
                    for (slot_idx, expr) in exprs.iter().enumerate() {
                        let folded = expr.fold_constants();
                        match folded {
                            HirExpr::Const(c) => {
                                out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::WriteFuncReturnSlot {
                                    func: current_fn_id,
                                    slot_idx: slot_idx as u32,
                                    src_var: None,
                                    const_val: Some(c),
                                });
                            }
                            HirExpr::Var(v) => {
                                out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::WriteFuncReturnSlot {
                                    func: current_fn_id,
                                    slot_idx: slot_idx as u32,
                                    src_var: Some(v),
                                    const_val: None,
                                });
                            }
                            _ => {
                                let tmp = scratch.acquire(hir);
                                lower_expr_into(&folded, tmp, hir, scratch, &mut out_blocks[cur_block.0 as usize].ops);
                                out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::WriteFuncReturnSlot {
                                    func: current_fn_id,
                                    slot_idx: slot_idx as u32,
                                    src_var: Some(tmp),
                                    const_val: None,
                                });
                                scratch.release(tmp);
                            }
                        }
                    }
                    out_blocks[cur_block.0 as usize].terminator = ExpandedTerminator::FunctionReturn(current_fn_id);
                    return Ok(None);
                }
                HirStmt::Break => {
                    let (_, break_target) = loop_ctx.ok_or_else(|| {
                        EudkitError::Ir("break statement outside of while loop".to_string())
                    })?;
                    out_blocks[cur_block.0 as usize].terminator =
                        ExpandedTerminator::Jump(break_target);
                    return Ok(None);
                }
                HirStmt::Continue => {
                    let (cond_target, _) = loop_ctx.ok_or_else(|| {
                        EudkitError::Ir("continue statement outside of while loop".to_string())
                    })?;
                    out_blocks[cur_block.0 as usize].terminator =
                        ExpandedTerminator::Jump(cond_target);
                    return Ok(None);
                }
                HirStmt::If {
                    conditions,
                    then_body,
                    else_body,
                } => {
                    let mut pre_ops = Vec::new();
                    let static_conds = prepare_conditions(conditions, hir, scratch, &mut pre_ops);
                    let had_dynamic_patch = pre_ops.iter().any(|op| {
                        matches!(
                            op,
                            ExpandedOp::PatchNextTriggerCond { .. }
                                | ExpandedOp::PatchNextTriggerAct { .. }
                        )
                    });
                    out_blocks[cur_block.0 as usize].ops.extend(pre_ops);

                    if !had_dynamic_patch && else_body.is_empty() && static_conds.len() <= 16 {
                        if let Some(leaf_actions) = extract_leaf_actions(then_body) {
                            if leaf_actions.is_empty() {
                                continue;
                            }
                            if leaf_actions.len() <= 64 {
                                out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::Trigger {
                                    label: Some("if_then".to_string()),
                                    conditions: static_conds,
                                    actions: leaf_actions,
                                    preserved: true,
                                });
                                continue;
                            }
                        }
                    } else if !had_dynamic_patch && static_conds.len() == 1 {
                        if let Some(inv_cond) = static_conds[0].invert() {
                            if let (Some(then_actions), Some(else_actions)) = (
                                extract_leaf_actions(then_body),
                                extract_leaf_actions(else_body),
                            ) {
                                if then_actions.len() <= 64
                                    && else_actions.len() <= 64
                                    && actions_cannot_mutate_condition(
                                        &then_actions,
                                        &static_conds[0],
                                    )
                                {
                                    if !then_actions.is_empty() {
                                        out_blocks[cur_block.0 as usize].ops.push(
                                            ExpandedOp::Trigger {
                                                label: Some("if_then_inplace".to_string()),
                                                conditions: static_conds,
                                                actions: then_actions,
                                                preserved: true,
                                            },
                                        );
                                    }
                                    if !else_actions.is_empty() {
                                        out_blocks[cur_block.0 as usize].ops.push(
                                            ExpandedOp::Trigger {
                                                label: Some("if_else_inplace".to_string()),
                                                conditions: vec![inv_cond],
                                                actions: else_actions,
                                                preserved: true,
                                            },
                                        );
                                    }
                                    continue;
                                }
                            }
                        }
                    }

                    let then_block = alloc_block(out_blocks, "if_then");
                    if else_body.is_empty() {
                        let merge_block = alloc_block(out_blocks, "if_merge");
                        out_blocks[cur_block.0 as usize].terminator = ExpandedTerminator::Branch {
                            conditions: static_conds,
                            then_block,
                            else_block: merge_block,
                        };
                        if let Some(then_end) =
                            lower_stmts(then_body, then_block, out_blocks, hir, scratch, loop_ctx, current_fn_id, fn_entry_block, fp_trampoline_block)?
                        {
                            out_blocks[then_end.0 as usize].terminator =
                                ExpandedTerminator::Jump(merge_block);
                        }
                        cur_block = merge_block;
                    } else {
                        let else_block = alloc_block(out_blocks, "if_else");
                        out_blocks[cur_block.0 as usize].terminator = ExpandedTerminator::Branch {
                            conditions: static_conds,
                            then_block,
                            else_block,
                        };
                        let then_end =
                            lower_stmts(then_body, then_block, out_blocks, hir, scratch, loop_ctx, current_fn_id, fn_entry_block, fp_trampoline_block)?;
                        let else_end =
                            lower_stmts(else_body, else_block, out_blocks, hir, scratch, loop_ctx, current_fn_id, fn_entry_block, fp_trampoline_block)?;
                        match (then_end, else_end) {
                            (None, None) => return Ok(None),
                            _ => {
                                let merge_block = alloc_block(out_blocks, "if_merge");
                                if let Some(te) = then_end {
                                    out_blocks[te.0 as usize].terminator =
                                        ExpandedTerminator::Jump(merge_block);
                                }
                                if let Some(ee) = else_end {
                                    out_blocks[ee.0 as usize].terminator =
                                        ExpandedTerminator::Jump(merge_block);
                                }
                                cur_block = merge_block;
                            }
                        }
                    }
                }
                HirStmt::While { conditions, body } => {
                    let cond_block = alloc_block(out_blocks, "while_cond");
                    let body_block = alloc_block(out_blocks, "while_body");
                    let exit_block = alloc_block(out_blocks, "while_exit");

                    out_blocks[cur_block.0 as usize].terminator =
                        ExpandedTerminator::Jump(cond_block);

                    let mut pre_ops = Vec::new();
                    let static_conds = prepare_conditions(conditions, hir, scratch, &mut pre_ops);
                    out_blocks[cond_block.0 as usize].ops.extend(pre_ops);
                    out_blocks[cond_block.0 as usize].terminator = ExpandedTerminator::Branch {
                        conditions: static_conds,
                        then_block: body_block,
                        else_block: exit_block,
                    };

                    if let Some(body_end) = lower_stmts(
                        body,
                        body_block,
                        out_blocks,
                        hir,
                        scratch,
                        Some((cond_block, exit_block)),
                        current_fn_id,
                        fn_entry_block,
                        fp_trampoline_block,
                    )? {
                        out_blocks[body_end.0 as usize].terminator =
                            ExpandedTerminator::Jump(cond_block);
                    }
                    cur_block = exit_block;
                }
            }
        }
        Ok(Some(cur_block))
    }

    let mut out_blocks: Vec<ExpandedBlock> = Vec::new();
    let mut block_map: HashMap<(crate::hir::FuncId, BlockId), BlockId> = HashMap::new();
    let mut fn_entry_block: HashMap<crate::hir::FuncId, BlockId> = HashMap::new();
    let mut fp_trampoline_block: HashMap<crate::hir::FuncPtrId, BlockId> = HashMap::new();

    // 1. Allocate blocks for main function first (so BlockId(0) is main entry)
    let main_func = hir.functions[main_fn_id.0 as usize].clone();
    for orig_b in &main_func.blocks {
        let new_id = BlockId(out_blocks.len() as u32);
        block_map.insert((main_fn_id, orig_b.id), new_id);
        out_blocks.push(ExpandedBlock {
            id: new_id,
            label: orig_b.label.clone(),
            ops: Vec::new(),
            terminator: ExpandedTerminator::Return,
        });
    }
    fn_entry_block.insert(main_fn_id, block_map[&(main_fn_id, main_func.blocks[0].id)]);

    // 2. Allocate blocks for all other functions
    for func in &hir.functions {
        if func.id == main_fn_id || func.blocks.is_empty() {
            continue;
        }
        for orig_b in &func.blocks {
            let new_id = BlockId(out_blocks.len() as u32);
            block_map.insert((func.id, orig_b.id), new_id);
            out_blocks.push(ExpandedBlock {
                id: new_id,
                label: format!("{}_{}", func.name, orig_b.label),
                ops: Vec::new(),
                terminator: ExpandedTerminator::FunctionReturn(func.id),
            });
        }
        fn_entry_block.insert(func.id, block_map[&(func.id, func.blocks[0].id)]);
    }

    // 3. Allocate trampoline blocks for all func_ptrs
    for fp in &hir.func_ptrs {
        let new_id = BlockId(out_blocks.len() as u32);
        let target_entry = fp
            .initial_target
            .and_then(|tgt| fn_entry_block.get(&tgt).copied())
            .unwrap_or(BlockId(0));
        out_blocks.push(ExpandedBlock {
            id: new_id,
            label: format!("fp_{}_trampoline", fp.name),
            ops: Vec::new(),
            terminator: ExpandedTerminator::Jump(target_entry),
        });
        fp_trampoline_block.insert(fp.id, new_id);
    }

    let all_functions = hir.functions.clone();
    for func in &all_functions {
        if func.blocks.is_empty() {
            continue;
        }
        for orig_block in &func.blocks {
            let cur_b = block_map[&(func.id, orig_block.id)];
            if let Some(end_block) = lower_stmts(
                &orig_block.stmts,
                cur_b,
                &mut out_blocks,
                hir,
                scratch,
                None,
                func.id,
                &fn_entry_block,
                &fp_trampoline_block,
            )? {
                match &orig_block.terminator {
                    HirTerminator::Jump(target) => {
                        let target_b = block_map[&(func.id, *target)];
                        out_blocks[end_block.0 as usize].terminator = ExpandedTerminator::Jump(target_b);
                    }
                    HirTerminator::Return => {
                        if func.id == main_fn_id {
                            out_blocks[end_block.0 as usize].terminator = ExpandedTerminator::Return;
                        } else {
                            out_blocks[end_block.0 as usize].terminator =
                                ExpandedTerminator::FunctionReturn(func.id);
                        }
                    }
                    HirTerminator::Branch {
                        conditions,
                        then_block,
                        else_block,
                    } => {
                        let mut pre_ops = Vec::new();
                        let static_conds = prepare_conditions(conditions, hir, scratch, &mut pre_ops);
                        out_blocks[end_block.0 as usize].ops.extend(pre_ops);
                        let then_b = block_map[&(func.id, *then_block)];
                        let else_b = block_map[&(func.id, *else_block)];
                        out_blocks[end_block.0 as usize].terminator = ExpandedTerminator::Branch {
                            conditions: static_conds,
                            then_block: then_b,
                            else_block: else_b,
                        };
                    }
                }
            }
        }
    }

    // Forward empty pass-through Jump blocks (protecting entry points)
    let protected_blocks: HashSet<BlockId> = std::iter::once(BlockId(0))
        .chain(fn_entry_block.values().copied())
        .chain(fp_trampoline_block.values().copied())
        .collect();

    let resolve_jump = |mut bid: BlockId, blocks: &[ExpandedBlock]| -> BlockId {
        for _ in 0..64 {
            if protected_blocks.contains(&bid) {
                break;
            }
            let blk = &blocks[bid.0 as usize];
            if blk.ops.is_empty() {
                if let ExpandedTerminator::Jump(next) = blk.terminator {
                    if next != bid {
                        bid = next;
                        continue;
                    }
                }
            }
            break;
        }
        bid
    };

    let snapshot = out_blocks.clone();
    for blk in &mut out_blocks {
        for op in &mut blk.ops {
            match op {
                ExpandedOp::CallSetup { return_block, .. }
                | ExpandedOp::CallIndirectSetup { return_block, .. } => {
                    *return_block = resolve_jump(*return_block, &snapshot);
                }
                _ => {}
            }
        }
        match &mut blk.terminator {
            ExpandedTerminator::Jump(target) => {
                *target = resolve_jump(*target, &snapshot);
            }
            ExpandedTerminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                *then_block = resolve_jump(*then_block, &snapshot);
                *else_block = resolve_jump(*else_block, &snapshot);
            }
            ExpandedTerminator::Return | ExpandedTerminator::FunctionReturn(_) => {}
        }
    }

    // Collect reachable blocks starting from BlockId(0)
    let mut reachable = vec![false; out_blocks.len()];
    let mut worklist = vec![BlockId(0)];
    reachable[0] = true;
    while let Some(bid) = worklist.pop() {
        let blk = &out_blocks[bid.0 as usize];
        for op in &blk.ops {
            match op {
                ExpandedOp::CallSetup { return_block, .. }
                | ExpandedOp::CallIndirectSetup { return_block, .. } => {
                    if !reachable[return_block.0 as usize] {
                        reachable[return_block.0 as usize] = true;
                        worklist.push(*return_block);
                    }
                }
                ExpandedOp::FuncPtrAssignTarget { target, .. } => {
                    let tgt_bid = fn_entry_block[target];
                    if !reachable[tgt_bid.0 as usize] {
                        reachable[tgt_bid.0 as usize] = true;
                        worklist.push(tgt_bid);
                    }
                }
                _ => {}
            }
        }
        match &blk.terminator {
            ExpandedTerminator::Jump(t) => {
                if !reachable[t.0 as usize] {
                    reachable[t.0 as usize] = true;
                    worklist.push(*t);
                }
            }
            ExpandedTerminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                if !reachable[then_block.0 as usize] {
                    reachable[then_block.0 as usize] = true;
                    worklist.push(*then_block);
                }
                if !reachable[else_block.0 as usize] {
                    reachable[else_block.0 as usize] = true;
                    worklist.push(*else_block);
                }
            }
            ExpandedTerminator::Return | ExpandedTerminator::FunctionReturn(_) => {}
        }
    }

    let mut old_to_new: HashMap<BlockId, BlockId> = HashMap::new();
    let mut compact_blocks: Vec<ExpandedBlock> = Vec::new();
    for (idx, mut blk) in out_blocks.into_iter().enumerate() {
        if reachable[idx] {
            let new_id = BlockId(compact_blocks.len() as u32);
            old_to_new.insert(blk.id, new_id);
            blk.id = new_id;
            compact_blocks.push(blk);
        }
    }
    for blk in &mut compact_blocks {
        for op in &mut blk.ops {
            match op {
                ExpandedOp::CallSetup { return_block, .. }
                | ExpandedOp::CallIndirectSetup { return_block, .. } => {
                    *return_block = old_to_new[return_block];
                }
                _ => {}
            }
        }
        match &mut blk.terminator {
            ExpandedTerminator::Jump(t) => *t = old_to_new[t],
            ExpandedTerminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                *then_block = old_to_new[then_block];
                *else_block = old_to_new[else_block];
            }
            ExpandedTerminator::Return | ExpandedTerminator::FunctionReturn(_) => {}
        }
    }

    let mut new_fn_entry_block = HashMap::new();
    for (fid, bid) in fn_entry_block {
        if let Some(&new_bid) = old_to_new.get(&bid) {
            new_fn_entry_block.insert(fid, new_bid);
        }
    }
    let mut new_fp_trampoline_block = HashMap::new();
    for (ptr, bid) in fp_trampoline_block {
        if let Some(&new_bid) = old_to_new.get(&bid) {
            new_fp_trampoline_block.insert(ptr, new_bid);
        }
    }

    Ok(ExpandedCfg {
        blocks: compact_blocks,
        fn_entry_block: new_fn_entry_block,
        fp_trampoline_block: new_fp_trampoline_block,
    })
}

#[derive(Debug, Clone, Copy)]
enum VarStorage {
    ByteBuf(ObjectId),
    VProcSlot { vbuf_obj: ObjectId, slot_idx: u32 },
}

impl VarStorage {
    fn value_epd(self) -> ValueExpr {
        match self {
            Self::ByteBuf(obj) => {
                ValueExpr::FieldEpd(FieldRef::new(obj, TriggerSlotField::ByteOffset(0)))
            }
            Self::VProcSlot { vbuf_obj, slot_idx } => ValueExpr::FieldEpd(FieldRef::vproc_slot(
                vbuf_obj.0,
                slot_idx,
                VProcSlotField::Value,
            )),
        }
    }

    fn value_address(self) -> ValueExpr {
        match self {
            Self::ByteBuf(obj) => ValueExpr::ObjectAddress {
                object: obj,
                byte_offset: 0,
            },
            Self::VProcSlot { vbuf_obj, slot_idx } => ValueExpr::FieldAddress(
                FieldRef::vproc_slot(vbuf_obj.0, slot_idx, VProcSlotField::Value),
            ),
        }
    }

    fn vproc_field(self, field: VProcSlotField) -> FieldRef {
        match self {
            Self::VProcSlot { vbuf_obj, slot_idx } => {
                FieldRef::vproc_slot(vbuf_obj.0, slot_idx, field)
            }
            Self::ByteBuf(_) => panic!("variable was not promoted to VProcBuffer"),
        }
    }
}

fn collect_vproc_vars(blocks: &[ExpandedBlock]) -> HashSet<VarId> {
    let mut set = HashSet::new();
    for blk in blocks {
        for op in &blk.ops {
            match *op {
                ExpandedOp::VProcCopyVar { src, .. }
                | ExpandedOp::VProcBitAndVar { src, .. }
                | ExpandedOp::VProcBitOrVar { src, .. }
                | ExpandedOp::VProcBitXorVar { src, .. }
                | ExpandedOp::PatchNextTriggerCond { src, .. }
                | ExpandedOp::PatchNextTriggerAct { src, .. } => {
                    set.insert(src);
                }
                ExpandedOp::DynamicShl { dst, .. } => {
                    set.insert(dst);
                }
                ExpandedOp::DynamicShr {
                    pow2_scratch,
                    div_x_scratch,
                    ..
                } => {
                    set.insert(pow2_scratch);
                    set.insert(div_x_scratch);
                }
                ExpandedOp::DynamicMul { rhs, .. } => {
                    set.insert(rhs);
                }
                ExpandedOp::DynamicDivRem {
                    divisor,
                    div_x_scratch,
                    ..
                } => {
                    set.insert(divisor);
                    set.insert(div_x_scratch);
                }
                ExpandedOp::WriteFuncReturnSlot { src_var: Some(src), .. } => {
                    set.insert(src);
                }
                ExpandedOp::LeafAction(_)
                | ExpandedOp::Trigger { .. }
                | ExpandedOp::CallSetup { .. }
                | ExpandedOp::CallIndirectSetup { .. }
                | ExpandedOp::WriteFuncReturnSlot { src_var: None, .. }
                | ExpandedOp::FuncPtrAssignTarget { .. } => {}
            }
        }
    }
    set
}

fn count_block_triggers(blk: &ExpandedBlock) -> u32 {
    let mut count = 0u32;
    let mut pending_actions = 0usize;

    let add_pending = |n: usize, pending: &mut usize, trigs: &mut u32| {
        if *pending + n > 64 {
            *trigs += 1;
            *pending = 0;
        }
        *pending += n;
        if *pending == 64 {
            *trigs += 1;
            *pending = 0;
        }
    };
    let flush_pending = |pending: &mut usize, trigs: &mut u32| {
        if *pending > 0 {
            *trigs += 1;
            *pending = 0;
        }
    };

    for op in &blk.ops {
        match op {
            ExpandedOp::LeafAction(_) => {
                add_pending(1, &mut pending_actions, &mut count);
            }
            ExpandedOp::Trigger { .. } => {
                flush_pending(&mut pending_actions, &mut count);
                count += 1;
            }
            ExpandedOp::VProcCopyVar { mask, .. } => {
                let n = if mask.is_some() { 4 } else { 3 };
                add_pending(n, &mut pending_actions, &mut count);
                flush_pending(&mut pending_actions, &mut count);
                if mask.is_some() {
                    add_pending(1, &mut pending_actions, &mut count);
                }
            }
            ExpandedOp::VProcBitAndVar { .. } => {
                add_pending(4, &mut pending_actions, &mut count);
                flush_pending(&mut pending_actions, &mut count);
                count += 1; // target masked SetDeathsX trigger
            }
            ExpandedOp::VProcBitOrVar { .. } => {
                add_pending(3, &mut pending_actions, &mut count);
                flush_pending(&mut pending_actions, &mut count);
                count += 1; // target masked SetDeathsX trigger
            }
            ExpandedOp::VProcBitXorVar { .. } => {
                add_pending(4, &mut pending_actions, &mut count);
                flush_pending(&mut pending_actions, &mut count);
                count += 1; // second pass with mask 0xAAAAAAAA
                add_pending(1, &mut pending_actions, &mut count);
            }
            ExpandedOp::DynamicShl { .. } => {
                add_pending(3, &mut pending_actions, &mut count);
                flush_pending(&mut pending_actions, &mut count);
                count += 2; // loopstart + loopcnt
                add_pending(1, &mut pending_actions, &mut count);
            }
            ExpandedOp::DynamicShr { .. } => {
                flush_pending(&mut pending_actions, &mut count);
                // 1 skip_check + 1 skip_to_here + 1 init pow2 + 32 exp2_vv + (1 init div + 32*5 + 32) = 229 triggers
                count += 2 + 1 + 32 + (1 + 32 * 5 + 32);
            }
            ExpandedOp::DynamicMul { .. } => {
                flush_pending(&mut pending_actions, &mut count);
                // 1 init + 32 * 2 + 31 * 2 + 1 endmul = 128 triggers
                count += 1 + 32 * 2 + 31 * 2 + 1;
            }
            ExpandedOp::DynamicDivRem { .. } => {
                flush_pending(&mut pending_actions, &mut count);
                // 1 init + 32 * 5 + 32 = 193 triggers
                count += 1 + 32 * 5 + 32;
            }
            ExpandedOp::PatchNextTriggerCond { .. } | ExpandedOp::PatchNextTriggerAct { .. } => {
                add_pending(3, &mut pending_actions, &mut count);
                flush_pending(&mut pending_actions, &mut count);
            }
            ExpandedOp::CallSetup { rets, .. } | ExpandedOp::CallIndirectSetup { rets, .. } => {
                let act_count = if rets.is_empty() { 1 } else { rets.len() + 1 };
                add_pending(act_count, &mut pending_actions, &mut count);
            }
            ExpandedOp::WriteFuncReturnSlot { src_var, const_val, .. } => {
                if const_val.is_some() {
                    add_pending(1, &mut pending_actions, &mut count);
                } else if src_var.is_some() {
                    add_pending(3, &mut pending_actions, &mut count);
                    flush_pending(&mut pending_actions, &mut count);
                }
            }
            ExpandedOp::FuncPtrAssignTarget { .. } => {
                add_pending(1, &mut pending_actions, &mut count);
            }
        }
    }

    if matches!(blk.terminator, ExpandedTerminator::Branch { .. }) {
        add_pending(1, &mut pending_actions, &mut count);
        flush_pending(&mut pending_actions, &mut count);
        count += 1; // branch_cond trigger
    } else {
        flush_pending(&mut pending_actions, &mut count);
        if count == 0 {
            count += 1;
        }
    }

    count
}

fn make_set_deaths_action(
    dest_epd: ValueExpr,
    modifier: Modifier,
    value: ValueExpr,
    mask: Option<u32>,
) -> LirAction {
    match mask {
        None => LirAction {
            locid1: ValueExpr::Const(0),
            strid: ValueExpr::Const(0),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: dest_epd,
            player2: value,
            unitid: 0,
            acttype: 45,
            amount: modifier.encode(),
            flags: 20,
            eudx: 0,
        },
        Some(m) => LirAction {
            locid1: ValueExpr::Const(m as i32),
            strid: ValueExpr::Const(0),
            wavid: ValueExpr::Const(0),
            time: ValueExpr::Const(0),
            player1: dest_epd,
            player2: value,
            unitid: 0,
            acttype: 45,
            amount: modifier.encode(),
            flags: 20,
            eudx: EUDX_SIGNATURE,
        },
    }
}

fn make_vproc_set_modifier(src_storage: VarStorage, modifier: Modifier) -> LirAction {
    let mod_field = src_storage.vproc_field(VProcSlotField::Modifier);
    let mode = (modifier.encode() as u32) << 24;
    make_set_deaths_action(
        ValueExpr::FieldEpd(mod_field),
        Modifier::SetTo,
        ValueExpr::Const(mode as i32),
        Some(0xFF00_0000),
    )
}

fn make_vproc_set_dest(src_storage: VarStorage, dest_epd: ValueExpr, modifier: Modifier) -> LirAction {
    let dest_field = src_storage.vproc_field(VProcSlotField::Dest);
    make_set_deaths_action(ValueExpr::FieldEpd(dest_field), modifier, dest_epd, None)
}

fn make_vproc_set_mask(src_storage: VarStorage, mask: u32) -> LirAction {
    let mask_field = src_storage.vproc_field(VProcSlotField::Mask);
    make_set_deaths_action(
        ValueExpr::FieldEpd(mask_field),
        Modifier::SetTo,
        ValueExpr::Const(mask as i32),
        None,
    )
}

fn make_vproc_set_nextptr(src_storage: VarStorage, next_addr: ValueExpr) -> LirAction {
    let nptr_field = src_storage.vproc_field(VProcSlotField::NextPtr);
    make_set_deaths_action(
        ValueExpr::FieldEpd(nptr_field),
        Modifier::SetTo,
        next_addr,
        None,
    )
}

fn vproc_trigger_addr(src_storage: VarStorage) -> ValueExpr {
    ValueExpr::FieldAddress(src_storage.vproc_field(VProcSlotField::TriggerBase))
}

pub fn lower_hir_to_lir(hir_in: &HirModule, map_ctx: &mut MapContext) -> Result<LirModule> {
    let mut hir = hir_in.clone();
    crate::inlining::inline_functions(&mut hir)?;
    let main_fn_id = hir
        .main_loop
        .or_else(|| hir.functions.first().map(|f| f.id))
        .ok_or_else(|| EudkitError::Ir("HIR module has no functions".to_string()))?;
    if hir.functions[main_fn_id.0 as usize].blocks.is_empty() {
        return Err(EudkitError::Ir(format!(
            "function '{}' has no basic blocks",
            hir.functions[main_fn_id.0 as usize].name
        )));
    }

    let mut scratch = ScratchAllocator::new();
    let expanded_cfg = lower_and_expand_cfg(&mut hir, main_fn_id, &mut scratch)?;
    let expanded_blocks = expanded_cfg.blocks;
    let fn_entry_block = expanded_cfg.fn_entry_block;
    let fp_trampoline_block = expanded_cfg.fp_trampoline_block;
    let vproc_var_set = collect_vproc_vars(&expanded_blocks);

    let has_ret_slots = hir.functions.iter().any(|f| f.return_count > 0 && fn_entry_block.contains_key(&f.id))
        || hir.func_ptrs.iter().any(|fp| fp.signature.1 > 0);
    let mut func_ret_slots: HashMap<crate::hir::FuncId, Vec<u32>> = HashMap::new();
    let mut ptr_ret_slots: HashMap<crate::hir::FuncPtrId, Vec<u32>> = HashMap::new();
    let mut fn_void_ret_trig: HashMap<crate::hir::FuncId, ObjectId> = HashMap::new();

    // Count exact triggers per block so Trigger 0 gets @obj0 and all ObjectIds are known up-front
    let mut trigger_count = 0u32;
    let mut block_first_trigger: HashMap<BlockId, ObjectId> = HashMap::new();
    for blk in &expanded_blocks {
        block_first_trigger.insert(blk.id, ObjectId(trigger_count));
        trigger_count += count_block_triggers(blk);
    }

    for func in &hir.functions {
        if func.id == main_fn_id || !fn_entry_block.contains_key(&func.id) {
            continue;
        }
        if func.return_count == 0 {
            let void_id = ObjectId(trigger_count);
            trigger_count += 1;
            fn_void_ret_trig.insert(func.id, void_id);
        }
    }

    let mut next_obj_idx = trigger_count;
    let vbuf_obj_id = if !vproc_var_set.is_empty() || has_ret_slots {
        let id = ObjectId(next_obj_idx);
        next_obj_idx += 1;
        Some(id)
    } else {
        None
    };

    let mut buf_id_to_obj: HashMap<ByteBufId, ObjectId> = HashMap::new();
    for buf in &hir.byte_bufs {
        buf_id_to_obj.insert(buf.id, ObjectId(next_obj_idx));
        next_obj_idx += 1;
    }

    // Partition variables into unified VProcBuffer slots vs 4-byte ByteBuf cells
    let mut var_storage: HashMap<VarId, VarStorage> = HashMap::new();
    let mut vproc_slots: Vec<LirVProcSlot> = Vec::new();
    let mut cell_vars: Vec<(ObjectId, &crate::hir::HirVar)> = Vec::new();

    for var in &hir.vars {
        if vproc_var_set.contains(&var.id) {
            let vbuf_obj = vbuf_obj_id.unwrap();
            let slot_idx = vproc_slots.len() as u32;
            vproc_slots.push(LirVProcSlot {
                label: Some(var.name.clone()),
                mask: ValueExpr::Const(-1),
                dest: ValueExpr::Const(0),
                value: ValueExpr::Const(var.init_val),
                modifier: Modifier::SetTo,
                next_ptr: ValueExpr::Const(0),
            });
            var_storage.insert(var.id, VarStorage::VProcSlot { vbuf_obj, slot_idx });
        } else {
            let obj_id = ObjectId(next_obj_idx);
            next_obj_idx += 1;
            var_storage.insert(var.id, VarStorage::ByteBuf(obj_id));
            cell_vars.push((obj_id, var));
        }
    }

    if let Some(vbuf) = vbuf_obj_id {
        // 1. Allocate shared return slots for each FuncPtr and map its targets to the same slots
        for fp in &hir.func_ptrs {
            let ret_count = fp.signature.1;
            if ret_count > 0 {
                let mut slots = Vec::new();
                for i in 0..ret_count {
                    let slot_idx = vproc_slots.len() as u32;
                    slots.push(slot_idx);
                    vproc_slots.push(LirVProcSlot {
                        label: Some(format!("fp_{}_ret_{}", fp.name, i)),
                        mask: ValueExpr::Const(-1),
                        dest: ValueExpr::Const(0),
                        value: ValueExpr::Const(0),
                        modifier: Modifier::SetTo,
                        next_ptr: ValueExpr::Const(0),
                    });
                }
                for i in 0..slots.len().saturating_sub(1) {
                    let next_slot = slots[i + 1];
                    vproc_slots[slots[i] as usize].next_ptr = ValueExpr::FieldAddress(FieldRef::vproc_slot(
                        vbuf.0,
                        next_slot,
                        VProcSlotField::TriggerBase,
                    ));
                }
                vproc_slots[*slots.last().unwrap() as usize].next_ptr = ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32);
                ptr_ret_slots.insert(fp.id, slots.clone());

                let mut fp_targets = HashSet::new();
                if let Some(target) = fp.initial_target {
                    fp_targets.insert(target);
                }
                for blk in &expanded_blocks {
                    for op in &blk.ops {
                        if let ExpandedOp::FuncPtrAssignTarget { ptr, target } = op {
                            if *ptr == fp.id {
                                fp_targets.insert(*target);
                            }
                        }
                    }
                }
                for tgt in fp_targets {
                    func_ret_slots.insert(tgt, slots.clone());
                }
            }
        }

        // 2. Allocate return slots for remaining non-FuncPtr functions
        for func in &hir.functions {
            if func.id == main_fn_id || !fn_entry_block.contains_key(&func.id) || func_ret_slots.contains_key(&func.id) {
                continue;
            }
            if func.return_count > 0 {
                let mut slots = Vec::new();
                for i in 0..func.return_count {
                    let slot_idx = vproc_slots.len() as u32;
                    slots.push(slot_idx);
                    vproc_slots.push(LirVProcSlot {
                        label: Some(format!("{}_ret_{}", func.name, i)),
                        mask: ValueExpr::Const(-1),
                        dest: ValueExpr::Const(0),
                        value: ValueExpr::Const(0),
                        modifier: Modifier::SetTo,
                        next_ptr: ValueExpr::Const(0),
                    });
                }
                for i in 0..slots.len().saturating_sub(1) {
                    let next_slot = slots[i + 1];
                    vproc_slots[slots[i] as usize].next_ptr = ValueExpr::FieldAddress(FieldRef::vproc_slot(
                        vbuf.0,
                        next_slot,
                        VProcSlotField::TriggerBase,
                    ));
                }
                vproc_slots[*slots.last().unwrap() as usize].next_ptr = ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32);
                func_ret_slots.insert(func.id, slots);
            }
        }
    }

    let lower_val = |v: HirValue| -> ValueExpr {
        match v {
            HirValue::Const(c) => ValueExpr::Const(c),
            HirValue::ByteBufAddress(bid, off) => ValueExpr::ObjectAddress {
                object: buf_id_to_obj[&bid],
                byte_offset: off as i32,
            },
            HirValue::ByteBufEpd(bid, off) => ValueExpr::ObjectEpd {
                object: buf_id_to_obj[&bid],
                dword_offset: off,
            },
            HirValue::VarAddress(vid) => var_storage[&vid].value_address(),
            HirValue::VarEpd(vid) => var_storage[&vid].value_epd(),
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
            StockCondition::VarCmp {
                var,
                comparison,
                value,
                mask,
            } => {
                let var_epd = var_storage[&var].value_epd();
                match mask {
                    None => LirCondition {
                        locid: ValueExpr::Const(0),
                        player: var_epd,
                        amount: ValueExpr::Const(value as i32),
                        unitid: 0,
                        comparison: comparison.encode(),
                        condtype: 15,
                        restype: 0,
                        flags: 0,
                        eudx: 0,
                    },
                    Some(m) => LirCondition {
                        locid: ValueExpr::Const(m as i32),
                        player: var_epd,
                        amount: ValueExpr::Const(value as i32),
                        unitid: 0,
                        comparison: comparison.encode(),
                        condtype: 15,
                        restype: 0,
                        flags: 0,
                        eudx: EUDX_SIGNATURE,
                    },
                }
            }
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
            StockCondition::DeathsDynamic { .. }
            | StockCondition::MemoryDynamic { .. }
            | StockCondition::ExprCmp { .. } => {
                unreachable!("prepare_conditions must lower all dynamic conditions")
            }
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
                player2: lower_val(*value),
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
                player2: lower_val(*value),
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
                player2: lower_val(*value),
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
                player2: lower_val(*value),
                unitid: 0,
                acttype: 45,
                amount: modifier.encode(),
                flags: 20,
                eudx: EUDX_SIGNATURE,
            },
            StockAction::SetVarConst {
                var,
                modifier,
                value,
                mask,
            } => {
                let var_epd = var_storage[var].value_epd();
                make_set_deaths_action(var_epd, *modifier, ValueExpr::Const(*value), *mask)
            }
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
                player2: lower_val(*value),
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
            StockAction::SetDeathsDynamic { .. }
            | StockAction::SetMemoryDynamic { .. }
            | StockAction::SetFieldDynamic { .. } => {
                unreachable!("lower_action_into_ops must lower all dynamic actions")
            }
        }
    };

    let mut objects: Vec<LirObject> = Vec::with_capacity(next_obj_idx as usize);
    let mut cur_obj_id = 0u32;

    for (blk_idx, block) in expanded_blocks.iter().enumerate() {
        debug_assert_eq!(ObjectId(cur_obj_id), block_first_trigger[&block.id]);
        let mut block_triggers: Vec<LirTrigger> = Vec::new();
        let mut pending_actions: Vec<LirAction> = Vec::new();

        // Helper: what is the address of the trigger that will run immediately after
        // a VProc jump if we flush `pending_actions` right now?
        // When `flush_with_custom_next` emits a trigger at `cur_obj_id`, the following
        // trigger in the block (or the block's terminator target if at the end of the block)
        // will be at `cur_obj_id + 1` unless the block has no more triggers!
        let flush_pending_normal = |pending: &mut Vec<LirAction>,
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
                    next_ptr: ValueExpr::ObjectAddress {
                        object: ObjectId(*cur_id),
                        byte_offset: 0,
                    },
                    conditions: Vec::new(),
                    actions: std::mem::take(pending),
                    flags: 4,
                });
            }
        };

        let ensure_capacity = |n: usize,
                               pending: &mut Vec<LirAction>,
                               out: &mut Vec<LirTrigger>,
                               cur_id: &mut u32,
                               blk_label: &str| {
            if pending.len() + n > 64 {
                flush_pending_normal(pending, out, cur_id, blk_label);
            }
        };

        let emit_custom_trigger = |label: String,
                                   next_ptr: ValueExpr,
                                   conditions: Vec<LirCondition>,
                                   actions: Vec<LirAction>,
                                   preserved: bool,
                                   out: &mut Vec<LirTrigger>,
                                   cur_id: &mut u32|
         -> ObjectId {
            let id = ObjectId(*cur_id);
            *cur_id += 1;
            out.push(LirTrigger {
                id,
                label: Some(label),
                prev_ptr: ValueExpr::Const(0),
                next_ptr,
                conditions,
                actions,
                flags: if preserved { 4 } else { 0 },
            });
            id
        };

        // Determine the address that the very last trigger of this block should jump to
        // if it is a VProc caller at the very end of the block (with Jump/Return terminator).
        let block_end_fallthrough = match &block.terminator {
            ExpandedTerminator::Jump(target_block) => ValueExpr::ObjectAddress {
                object: block_first_trigger[target_block],
                byte_offset: 0,
            },
            ExpandedTerminator::Return => ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
            ExpandedTerminator::FunctionReturn(fid) => {
                if let Some(slots) = func_ret_slots.get(fid) {
                    if !slots.is_empty() {
                        let vbuf = vbuf_obj_id.unwrap();
                        ValueExpr::FieldAddress(FieldRef::vproc_slot(
                            vbuf.0,
                            slots[0],
                            VProcSlotField::TriggerBase,
                        ))
                    } else if let Some(&trig) = fn_void_ret_trig.get(fid) {
                        ValueExpr::ObjectAddress {
                            object: trig,
                            byte_offset: 0,
                        }
                    } else {
                        ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                    }
                } else if let Some(&trig) = fn_void_ret_trig.get(fid) {
                    ValueExpr::ObjectAddress {
                        object: trig,
                        byte_offset: 0,
                    }
                } else {
                    ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                }
            }
            ExpandedTerminator::Branch { .. } => {
                // A Branch terminator always emits at least 2 triggers at the end of the block,
                // so any VProc op in `block.ops` is guaranteed to be followed by another trigger in this block!
                ValueExpr::Const(0)
            }
        };
        let block_end_obj_id = if blk_idx + 1 < expanded_blocks.len() {
            block_first_trigger[&expanded_blocks[blk_idx + 1].id].0
        } else {
            trigger_count
        };

        let next_return_addr = |after_trig_id: u32| -> ValueExpr {
            if after_trig_id < block_end_obj_id {
                ValueExpr::ObjectAddress {
                    object: ObjectId(after_trig_id),
                    byte_offset: 0,
                }
            } else {
                block_end_fallthrough
            }
        };

        for op in &block.ops {
            match op {
                ExpandedOp::LeafAction(act) => {
                    ensure_capacity(
                        1,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    pending_actions.push(lower_act(act));
                    if pending_actions.len() == 64 {
                        flush_pending_normal(
                            &mut pending_actions,
                            &mut block_triggers,
                            &mut cur_obj_id,
                            &block.label,
                        );
                    }
                }
                ExpandedOp::Trigger {
                    label,
                    conditions,
                    actions,
                    preserved,
                } => {
                    flush_pending_normal(
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
                    let next_id = cur_obj_id + 1;
                    let lbl = label
                        .clone()
                        .unwrap_or_else(|| format!("{}_{}", block.label, cur_obj_id));
                    emit_custom_trigger(
                        lbl,
                        next_return_addr(next_id),
                        conditions.iter().map(&lower_cond).collect(),
                        actions.iter().map(&mut lower_act).collect(),
                        *preserved,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                }
                ExpandedOp::VProcCopyVar {
                    src,
                    dst,
                    modifier,
                    mask,
                } => {
                    let src_st = var_storage[src];
                    let dst_epd = var_storage[dst].value_epd();
                    let need_acts = if mask.is_some() { 4 } else { 3 };
                    ensure_capacity(
                        need_acts,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    if let Some(m) = mask {
                        pending_actions.push(make_vproc_set_mask(src_st, *m));
                    }
                    pending_actions.push(make_vproc_set_dest(src_st, dst_epd, Modifier::SetTo));
                    pending_actions.push(make_vproc_set_modifier(src_st, *modifier));
                    pending_actions.push(make_vproc_set_nextptr(src_st, ret_addr));

                    let acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_vproc_copy_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    if mask.is_some() {
                        pending_actions.push(make_vproc_set_mask(src_st, 0xFFFF_FFFF));
                    }
                }
                ExpandedOp::VProcBitAndVar { src, dst } => {
                    let src_st = var_storage[src];
                    let dst_epd = var_storage[dst].value_epd();
                    ensure_capacity(
                        4,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let mask_trig_id = ObjectId(cur_obj_id + 1);
                    let mask_field_epd = ValueExpr::FieldEpd(FieldRef::action(
                        mask_trig_id.0,
                        0,
                        ActField::LocId1,
                    ));
                    // mask_field = ~0; mask_field -= src
                    pending_actions.push(make_set_deaths_action(
                        mask_field_epd,
                        Modifier::SetTo,
                        ValueExpr::Const(-1),
                        None,
                    ));
                    pending_actions.push(make_vproc_set_dest(
                        src_st,
                        mask_field_epd,
                        Modifier::SetTo,
                    ));
                    pending_actions.push(make_vproc_set_modifier(src_st, Modifier::Subtract));
                    pending_actions.push(make_vproc_set_nextptr(
                        src_st,
                        ValueExpr::ObjectAddress {
                            object: mask_trig_id,
                            byte_offset: 0,
                        },
                    ));
                    let setup_acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_bitand_setup_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        setup_acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    debug_assert_eq!(ObjectId(cur_obj_id), mask_trig_id);
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    emit_custom_trigger(
                        format!("{}_bitand_apply_{}", block.label, cur_obj_id),
                        ret_addr,
                        Vec::new(),
                        vec![make_set_deaths_action(
                            dst_epd,
                            Modifier::SetTo,
                            ValueExpr::Const(0),
                            Some(0xFFFF_FFFF),
                        )],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                }
                ExpandedOp::VProcBitOrVar { src, dst } => {
                    let src_st = var_storage[src];
                    let dst_epd = var_storage[dst].value_epd();
                    ensure_capacity(
                        3,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let mask_trig_id = ObjectId(cur_obj_id + 1);
                    let mask_field_epd = ValueExpr::FieldEpd(FieldRef::action(
                        mask_trig_id.0,
                        0,
                        ActField::LocId1,
                    ));
                    pending_actions.push(make_vproc_set_dest(
                        src_st,
                        mask_field_epd,
                        Modifier::SetTo,
                    ));
                    pending_actions.push(make_vproc_set_modifier(src_st, Modifier::SetTo));
                    pending_actions.push(make_vproc_set_nextptr(
                        src_st,
                        ValueExpr::ObjectAddress {
                            object: mask_trig_id,
                            byte_offset: 0,
                        },
                    ));
                    let setup_acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_bitor_setup_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        setup_acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    debug_assert_eq!(ObjectId(cur_obj_id), mask_trig_id);
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    emit_custom_trigger(
                        format!("{}_bitor_apply_{}", block.label, cur_obj_id),
                        ret_addr,
                        Vec::new(),
                        vec![make_set_deaths_action(
                            dst_epd,
                            Modifier::SetTo,
                            ValueExpr::Const(-1),
                            Some(0),
                        )],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                }
                ExpandedOp::VProcBitXorVar { src, dst } => {
                    let src_st = var_storage[src];
                    let dst_epd = var_storage[dst].value_epd();
                    ensure_capacity(
                        4,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let pass2_trig_id = ObjectId(cur_obj_id + 1);
                    pending_actions.push(make_vproc_set_mask(src_st, 0x5555_5555));
                    pending_actions.push(make_vproc_set_dest(src_st, dst_epd, Modifier::SetTo));
                    pending_actions.push(make_vproc_set_modifier(src_st, Modifier::Add));
                    pending_actions.push(make_vproc_set_nextptr(
                        src_st,
                        ValueExpr::ObjectAddress {
                            object: pass2_trig_id,
                            byte_offset: 0,
                        },
                    ));
                    let pass1_acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_bitxor_p1_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        pass1_acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    debug_assert_eq!(ObjectId(cur_obj_id), pass2_trig_id);
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    emit_custom_trigger(
                        format!("{}_bitxor_p2_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        vec![
                            make_vproc_set_mask(src_st, 0xAAAA_AAAA),
                            make_vproc_set_nextptr(src_st, ret_addr),
                        ],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    pending_actions.push(make_vproc_set_mask(src_st, 0xFFFF_FFFF));
                }
                ExpandedOp::DynamicShl { dst, shift } => {
                    let dst_st = var_storage[dst];
                    let dst_epd = dst_st.value_epd();
                    ensure_capacity(
                        3,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let loopstart_id = ObjectId(cur_obj_id + 1);
                    let loopcnt_id = ObjectId(cur_obj_id + 2);
                    let loopend_addr = next_return_addr(cur_obj_id + 3);

                    pending_actions.push(make_vproc_set_nextptr(
                        dst_st,
                        ValueExpr::ObjectAddress {
                            object: loopcnt_id,
                            byte_offset: 0,
                        },
                    ));
                    pending_actions.push(make_vproc_set_dest(dst_st, dst_epd, Modifier::SetTo));
                    pending_actions.push(make_vproc_set_modifier(dst_st, Modifier::Add));
                    let setup_acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_dyn_shl_setup_{}", block.label, cur_obj_id),
                        ValueExpr::ObjectAddress {
                            object: loopstart_id,
                            byte_offset: 0,
                        },
                        Vec::new(),
                        setup_acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    debug_assert_eq!(ObjectId(cur_obj_id), loopstart_id);
                    let loopstart_nptr_epd = ValueExpr::FieldEpd(FieldRef::new(
                        loopstart_id,
                        TriggerSlotField::NextPtr,
                    ));
                    emit_custom_trigger(
                        format!("{}_dyn_shl_loopstart_{}", block.label, cur_obj_id),
                        loopend_addr,
                        vec![
                            lower_cond(&StockCondition::VarCmp {
                                var: *dst,
                                comparison: Comparison::AtLeast,
                                value: 1,
                                mask: None,
                            }),
                            lower_cond(&StockCondition::VarCmp {
                                var: *shift,
                                comparison: Comparison::AtLeast,
                                value: 1,
                                mask: None,
                            }),
                        ],
                        vec![make_set_deaths_action(
                            loopstart_nptr_epd,
                            Modifier::SetTo,
                            vproc_trigger_addr(dst_st),
                            None,
                        )],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    debug_assert_eq!(ObjectId(cur_obj_id), loopcnt_id);
                    emit_custom_trigger(
                        format!("{}_dyn_shl_loopcnt_{}", block.label, cur_obj_id),
                        ValueExpr::ObjectAddress {
                            object: loopstart_id,
                            byte_offset: 0,
                        },
                        Vec::new(),
                        vec![
                            lower_act(&StockAction::SetVarConst {
                                var: *shift,
                                modifier: Modifier::Subtract,
                                value: 1,
                                mask: None,
                            }),
                            make_set_deaths_action(
                                loopstart_nptr_epd,
                                Modifier::SetTo,
                                loopend_addr,
                                None,
                            ),
                        ],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    pending_actions.push(make_vproc_set_modifier(dst_st, Modifier::SetTo));
                }
                ExpandedOp::DynamicMul { lhs, rhs, dst } => {
                    flush_pending_normal(
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let rhs_st = var_storage[rhs];
                    let rhs_epd = rhs_st.value_epd();
                    let dst_epd = var_storage[dst].value_epd();

                    // Total triggers emitted by DynamicMul: 1 + 32*2 + 31*2 + 1 = 128
                    let endmul_id = ObjectId(cur_obj_id + 127);
                    let reset_p1_epd = ValueExpr::FieldEpd(FieldRef::action(
                        endmul_id.0,
                        0,
                        ActField::Player1,
                    ));
                    let reset_p2_epd = ValueExpr::FieldEpd(FieldRef::action(
                        endmul_id.0,
                        0,
                        ActField::Player2,
                    ));

                    // 1. Init trigger
                    emit_custom_trigger(
                        format!("{}_mul_init_{}", block.label, cur_obj_id),
                        ValueExpr::ObjectAddress {
                            object: ObjectId(cur_obj_id + 1),
                            byte_offset: 0,
                        },
                        Vec::new(),
                        vec![
                            make_set_deaths_action(
                                dst_epd,
                                Modifier::SetTo,
                                ValueExpr::Const(0),
                                None,
                            ),
                            make_vproc_set_modifier(rhs_st, Modifier::Add),
                        ],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );

                    let mut remaining_bits = 0xFFFF_FFFFu32;
                    let mut first_p1 = ObjectId(0);
                    let mut first_p2 = ObjectId(0);

                    for i in 0..32u32 {
                        let bit = 1u32 << i;
                        remaining_bits &= !bit;
                        let p1_id = ObjectId(cur_obj_id);
                        let p3_id = ObjectId(cur_obj_id + 1);
                        let p2_id = ObjectId(cur_obj_id + 2);
                        if i == 0 {
                            first_p1 = p1_id;
                            first_p2 = p2_id;
                        }
                        let p1_nptr_epd = ValueExpr::FieldEpd(FieldRef::new(
                            p1_id,
                            TriggerSlotField::NextPtr,
                        ));
                        // p1: if lhs & (1<<i), jump to p3 else p2
                        emit_custom_trigger(
                            format!("{}_mul_p1_{i}_{}", block.label, cur_obj_id),
                            ValueExpr::ObjectAddress {
                                object: p2_id,
                                byte_offset: 0,
                            },
                            vec![lower_cond(&StockCondition::VarCmp {
                                var: *lhs,
                                comparison: Comparison::AtLeast,
                                value: 1,
                                mask: Some(bit),
                            })],
                            vec![make_set_deaths_action(
                                p1_nptr_epd,
                                Modifier::SetTo,
                                ValueExpr::ObjectAddress {
                                    object: p3_id,
                                    byte_offset: 0,
                                },
                                None,
                            )],
                            true,
                            &mut block_triggers,
                            &mut cur_obj_id,
                        );
                        // p3: VProc(rhs -> dst), restore p1.nextptr = p2, return to p2
                        emit_custom_trigger(
                            format!("{}_mul_p3_{i}_{}", block.label, cur_obj_id),
                            vproc_trigger_addr(rhs_st),
                            Vec::new(),
                            vec![
                                make_vproc_set_dest(rhs_st, dst_epd, Modifier::SetTo),
                                make_set_deaths_action(
                                    p1_nptr_epd,
                                    Modifier::SetTo,
                                    ValueExpr::ObjectAddress {
                                        object: p2_id,
                                        byte_offset: 0,
                                    },
                                    None,
                                ),
                                make_vproc_set_nextptr(
                                    rhs_st,
                                    ValueExpr::ObjectAddress {
                                        object: p2_id,
                                        byte_offset: 0,
                                    },
                                ),
                            ],
                            true,
                            &mut block_triggers,
                            &mut cur_obj_id,
                        );

                        if i <= 30 {
                            let p4_id = ObjectId(cur_obj_id + 1);
                            let next_iter_id = ObjectId(cur_obj_id + 2);
                            let p2_nptr_epd = ValueExpr::FieldEpd(FieldRef::new(
                                p2_id,
                                TriggerSlotField::NextPtr,
                            ));
                            // p2: early exit check if remaining_bits == 0
                            emit_custom_trigger(
                                format!("{}_mul_p2_{i}_{}", block.label, cur_obj_id),
                                ValueExpr::ObjectAddress {
                                    object: p4_id,
                                    byte_offset: 0,
                                },
                                vec![lower_cond(&StockCondition::VarCmp {
                                    var: *lhs,
                                    comparison: Comparison::Exactly,
                                    value: 0,
                                    mask: Some(remaining_bits),
                                })],
                                vec![
                                    make_set_deaths_action(
                                        p2_nptr_epd,
                                        Modifier::SetTo,
                                        ValueExpr::ObjectAddress {
                                            object: endmul_id,
                                            byte_offset: 0,
                                        },
                                        None,
                                    ),
                                    make_set_deaths_action(
                                        reset_p1_epd,
                                        Modifier::SetTo,
                                        p2_nptr_epd,
                                        None,
                                    ),
                                    make_set_deaths_action(
                                        reset_p2_epd,
                                        Modifier::SetTo,
                                        ValueExpr::ObjectAddress {
                                            object: p4_id,
                                            byte_offset: 0,
                                        },
                                        None,
                                    ),
                                ],
                                true,
                                &mut block_triggers,
                                &mut cur_obj_id,
                            );
                            // p4: VProc(rhs -> rhs) (rhs += rhs), return to next_iter_id
                            emit_custom_trigger(
                                format!("{}_mul_p4_{i}_{}", block.label, cur_obj_id),
                                vproc_trigger_addr(rhs_st),
                                Vec::new(),
                                vec![
                                    make_vproc_set_dest(rhs_st, rhs_epd, Modifier::SetTo),
                                    make_vproc_set_nextptr(
                                        rhs_st,
                                        ValueExpr::ObjectAddress {
                                            object: next_iter_id,
                                            byte_offset: 0,
                                        },
                                    ),
                                ],
                                true,
                                &mut block_triggers,
                                &mut cur_obj_id,
                            );
                        }
                    }

                    debug_assert_eq!(ObjectId(cur_obj_id), endmul_id);
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    let first_p1_nptr_epd = ValueExpr::FieldEpd(FieldRef::new(
                        first_p1,
                        TriggerSlotField::NextPtr,
                    ));
                    emit_custom_trigger(
                        format!("{}_mul_end_{}", block.label, cur_obj_id),
                        ret_addr,
                        Vec::new(),
                        vec![
                            make_set_deaths_action(
                                first_p1_nptr_epd,
                                Modifier::SetTo,
                                ValueExpr::ObjectAddress {
                                    object: first_p2,
                                    byte_offset: 0,
                                },
                                None,
                            ),
                            make_vproc_set_modifier(rhs_st, Modifier::SetTo),
                        ],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                }
                ExpandedOp::DynamicDivRem {
                    dividend,
                    divisor,
                    quot_dst,
                    div_x_scratch,
                } => {
                    flush_pending_normal(
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    emit_dynamic_div_chain(
                        *dividend,
                        *divisor,
                        *quot_dst,
                        *div_x_scratch,
                        &var_storage,
                        &lower_cond,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                        &next_return_addr,
                    );
                }
                ExpandedOp::DynamicShr {
                    dst,
                    shift,
                    pow2_scratch,
                    div_x_scratch,
                    div_q_scratch,
                } => {
                    flush_pending_normal(
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    // Total triggers: 2 + 1 + 32 + 193 = 228 triggers? Wait:
                    // In count_block_triggers we counted 2 + 1 + 32 + (1 + 32*5 + 32) = 228 triggers!
                    let skip_check_id = ObjectId(cur_obj_id);
                    let skip_to_here_id = ObjectId(cur_obj_id + 1);
                    let exponential_id = ObjectId(cur_obj_id + 2);
                    let after_all_id = cur_obj_id + 2 + 1 + 32 + 193;
                    let ret_addr = next_return_addr(after_all_id);
                    let skip_check_nptr_epd = ValueExpr::FieldEpd(FieldRef::new(
                        skip_check_id,
                        TriggerSlotField::NextPtr,
                    ));
                    let dst_epd = var_storage[dst].value_epd();
                    let pow2_epd = var_storage[pow2_scratch].value_epd();

                    // 1. skip_check: if shift >= 32, dst = 0 and jump to skip_to_here
                    emit_custom_trigger(
                        format!("{}_shr_skip_check_{}", block.label, cur_obj_id),
                        ValueExpr::ObjectAddress {
                            object: exponential_id,
                            byte_offset: 0,
                        },
                        vec![lower_cond(&StockCondition::VarCmp {
                            var: *shift,
                            comparison: Comparison::AtLeast,
                            value: 32,
                            mask: None,
                        })],
                        vec![
                            make_set_deaths_action(
                                dst_epd,
                                Modifier::SetTo,
                                ValueExpr::Const(0),
                                None,
                            ),
                            make_set_deaths_action(
                                skip_check_nptr_epd,
                                Modifier::SetTo,
                                ValueExpr::ObjectAddress {
                                    object: skip_to_here_id,
                                    byte_offset: 0,
                                },
                                None,
                            ),
                        ],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    // 2. skip_to_here: restore skip_check.nextptr = exponential and jump to ret_addr
                    emit_custom_trigger(
                        format!("{}_shr_skip_to_here_{}", block.label, cur_obj_id),
                        ret_addr,
                        Vec::new(),
                        vec![make_set_deaths_action(
                            skip_check_nptr_epd,
                            Modifier::SetTo,
                            ValueExpr::ObjectAddress {
                                object: exponential_id,
                                byte_offset: 0,
                            },
                            None,
                        )],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    // 3. init pow2_scratch = 0
                    emit_custom_trigger(
                        format!("{}_shr_exp2_init_{}", block.label, cur_obj_id),
                        ValueExpr::ObjectAddress {
                            object: ObjectId(cur_obj_id + 1),
                            byte_offset: 0,
                        },
                        Vec::new(),
                        vec![make_set_deaths_action(
                            pow2_epd,
                            Modifier::SetTo,
                            ValueExpr::Const(0),
                            None,
                        )],
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                    // 4. 32 triggers: if shift == i, pow2_scratch = 1 << i
                    for i in 0..32u32 {
                        emit_custom_trigger(
                            format!("{}_shr_exp2_{i}_{}", block.label, cur_obj_id),
                            ValueExpr::ObjectAddress {
                                object: ObjectId(cur_obj_id + 1),
                                byte_offset: 0,
                            },
                            vec![lower_cond(&StockCondition::VarCmp {
                                var: *shift,
                                comparison: Comparison::Exactly,
                                value: i,
                                mask: None,
                            })],
                            vec![make_set_deaths_action(
                                pow2_epd,
                                Modifier::SetTo,
                                ValueExpr::Const((1u32 << i) as i32),
                                None,
                            )],
                            true,
                            &mut block_triggers,
                            &mut cur_obj_id,
                        );
                    }
                    // 5. Dynamic division of dst by pow2_scratch -> quotient into div_q_scratch,
                    // wait: we want quotient in `dst`! Since `emit_dynamic_div_chain` writes quotient
                    // into `quot_dst` while mutating `dividend`, we pass `dividend = *div_q_scratch`
                    // if we copied `dst` there, OR we pass `dividend = *dst, quot_dst = *div_q_scratch`?
                    // Wait: if we pass `dividend = *dst, quot_dst = *dst`, `quot_dst` is zeroed at the start!
                    // So in `DynamicShr`, `lower_expr_into` can put `lhs` into `div_q_scratch` or we copy `div_q_scratch`!
                    // Wait: in `emit_dynamic_div_chain`, the first init trigger has 2 VProc jumps? No, 1 VProc jump (copy divisor -> div_x_scratch) and 1 constant action `quot_dst = 0`!
                    // If `DynamicShr` puts `lhs` into `div_q_scratch` and passes `dividend = *div_q_scratch, quot_dst = *dst`, then `dst` gets the quotient directly with 0 extra triggers!
                    emit_dynamic_div_chain(
                        *div_q_scratch,
                        *pow2_scratch,
                        *dst,
                        *div_x_scratch,
                        &var_storage,
                        &lower_cond,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                        &next_return_addr,
                    );
                }
                ExpandedOp::PatchNextTriggerCond {
                    src,
                    cond_idx,
                    field,
                    is_epd,
                } => {
                    // Find the target trigger ID: after flushing all pending PatchNextTrigger* ops,
                    // what trigger ID will the patched trigger have?
                    //Wait: each PatchNextTrigger* op flushes into 1 trigger!
                    // How many more PatchNextTrigger* ops follow consecutively before the target Trigger/Branch?
                    let src_st = var_storage[src];
                    ensure_capacity(
                        3,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let _ = is_epd;
                    // We will patch the target trigger ID in a second pass or compute the offset!
                    // Wait: let's see! If there are K consecutive PatchNextTrigger* ops, they emit K triggers,
                    // and the target trigger is immediately after those K triggers (or if before a Branch terminator,
                    // after the 1 branch_reset trigger, so at +1)!
                    // Even simpler: we can record a fixup `(trigger_vec_idx, act_idx_for_dest, cond_idx, field)`
                    // and resolve it as soon as the next real Trigger (or Branch cond trigger) is emitted!
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    pending_actions.push(make_vproc_set_dest(
                        src_st,
                        ValueExpr::FieldEpd(FieldRef::condition(
                            u32::MAX, // placeholder resolved when target trigger is emitted
                            *cond_idx,
                            *field,
                        )),
                        Modifier::SetTo,
                    ));
                    pending_actions.push(make_vproc_set_modifier(src_st, Modifier::SetTo));
                    pending_actions.push(make_vproc_set_nextptr(src_st, ret_addr));
                    let acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_patch_cond_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                }
                ExpandedOp::PatchNextTriggerAct {
                    src,
                    act_idx,
                    field,
                    is_epd,
                } => {
                    let src_st = var_storage[src];
                    ensure_capacity(
                        3,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let _ = is_epd;
                    let ret_addr = next_return_addr(cur_obj_id + 1);
                    pending_actions.push(make_vproc_set_dest(
                        src_st,
                        ValueExpr::FieldEpd(FieldRef::action(
                            u32::MAX, // placeholder resolved when target trigger is emitted
                            *act_idx,
                            *field,
                        )),
                        Modifier::SetTo,
                    ));
                    pending_actions.push(make_vproc_set_modifier(src_st, Modifier::SetTo));
                    pending_actions.push(make_vproc_set_nextptr(src_st, ret_addr));
                    let acts = std::mem::take(&mut pending_actions);
                    emit_custom_trigger(
                        format!("{}_patch_act_{}", block.label, cur_obj_id),
                        vproc_trigger_addr(src_st),
                        Vec::new(),
                        acts,
                        true,
                        &mut block_triggers,
                        &mut cur_obj_id,
                    );
                }
                ExpandedOp::CallSetup {
                    callee,
                    rets,
                    return_block,
                } => {
                    let ret_site = block_first_trigger[return_block];
                    let ret_site_addr = ValueExpr::ObjectAddress {
                        object: ret_site,
                        byte_offset: 0,
                    };
                    if let Some(slots) = func_ret_slots.get(callee) {
                        if !slots.is_empty() {
                            let vbuf = vbuf_obj_id.unwrap();
                            let need = rets.len() + 1;
                            ensure_capacity(
                                need,
                                &mut pending_actions,
                                &mut block_triggers,
                                &mut cur_obj_id,
                                &block.label,
                            );
                            for (k, dst_var) in rets.iter().enumerate() {
                                if k < slots.len() {
                                    let slot_idx = slots[k];
                                    let dest_field_epd = ValueExpr::FieldEpd(FieldRef::vproc_slot(
                                        vbuf.0,
                                        slot_idx,
                                        VProcSlotField::Dest,
                                    ));
                                    let dst_epd = var_storage[dst_var].value_epd();
                                    pending_actions.push(make_set_deaths_action(
                                        dest_field_epd,
                                        Modifier::SetTo,
                                        dst_epd,
                                        None,
                                    ));
                                }
                            }
                            let last_slot = *slots.last().unwrap();
                            let next_field_epd = ValueExpr::FieldEpd(FieldRef::vproc_slot(
                                vbuf.0,
                                last_slot,
                                VProcSlotField::NextPtr,
                            ));
                            pending_actions.push(make_set_deaths_action(
                                next_field_epd,
                                Modifier::SetTo,
                                ret_site_addr,
                                None,
                            ));
                        } else if let Some(&void_trig) = fn_void_ret_trig.get(callee) {
                            ensure_capacity(
                                1,
                                &mut pending_actions,
                                &mut block_triggers,
                                &mut cur_obj_id,
                                &block.label,
                            );
                            let next_field_epd = ValueExpr::FieldEpd(FieldRef::new(void_trig, TriggerSlotField::NextPtr));
                            pending_actions.push(make_set_deaths_action(
                                next_field_epd,
                                Modifier::SetTo,
                                ret_site_addr,
                                None,
                            ));
                        }
                    } else if let Some(&void_trig) = fn_void_ret_trig.get(callee) {
                        ensure_capacity(
                            1,
                            &mut pending_actions,
                            &mut block_triggers,
                            &mut cur_obj_id,
                            &block.label,
                        );
                        let next_field_epd = ValueExpr::FieldEpd(FieldRef::new(void_trig, TriggerSlotField::NextPtr));
                        pending_actions.push(make_set_deaths_action(
                            next_field_epd,
                            Modifier::SetTo,
                            ret_site_addr,
                            None,
                        ));
                    }
                }
                ExpandedOp::CallIndirectSetup {
                    ptr,
                    rets,
                    return_block,
                } => {
                    let ret_site = block_first_trigger[return_block];
                    let ret_site_addr = ValueExpr::ObjectAddress {
                        object: ret_site,
                        byte_offset: 0,
                    };
                    if let Some(slots) = ptr_ret_slots.get(ptr) {
                        if !slots.is_empty() {
                            let vbuf = vbuf_obj_id.unwrap();
                            let need = rets.len() + 1;
                            ensure_capacity(
                                need,
                                &mut pending_actions,
                                &mut block_triggers,
                                &mut cur_obj_id,
                                &block.label,
                            );
                            for (k, dst_var) in rets.iter().enumerate() {
                                if k < slots.len() {
                                    let slot_idx = slots[k];
                                    let dest_field_epd = ValueExpr::FieldEpd(FieldRef::vproc_slot(
                                        vbuf.0,
                                        slot_idx,
                                        VProcSlotField::Dest,
                                    ));
                                    let dst_epd = var_storage[dst_var].value_epd();
                                    pending_actions.push(make_set_deaths_action(
                                        dest_field_epd,
                                        Modifier::SetTo,
                                        dst_epd,
                                        None,
                                    ));
                                }
                            }
                            let last_slot = *slots.last().unwrap();
                            let next_field_epd = ValueExpr::FieldEpd(FieldRef::vproc_slot(
                                vbuf.0,
                                last_slot,
                                VProcSlotField::NextPtr,
                            ));
                            pending_actions.push(make_set_deaths_action(
                                next_field_epd,
                                Modifier::SetTo,
                                ret_site_addr,
                                None,
                            ));
                        }
                    }
                }
                ExpandedOp::WriteFuncReturnSlot {
                    func,
                    slot_idx,
                    src_var,
                    const_val,
                } => {
                    let slots = &func_ret_slots[func];
                    let actual_slot = slots[*slot_idx as usize];
                    let vbuf = vbuf_obj_id.unwrap();
                    let val_field_epd = ValueExpr::FieldEpd(FieldRef::vproc_slot(
                        vbuf.0,
                        actual_slot,
                        VProcSlotField::Value,
                    ));
                    if let Some(c) = const_val {
                        ensure_capacity(
                            1,
                            &mut pending_actions,
                            &mut block_triggers,
                            &mut cur_obj_id,
                            &block.label,
                        );
                        pending_actions.push(make_set_deaths_action(
                            val_field_epd,
                            Modifier::SetTo,
                            ValueExpr::Const(*c),
                            None,
                        ));
                    } else if let Some(src) = src_var {
                        let src_st = var_storage[src];
                        ensure_capacity(
                            3,
                            &mut pending_actions,
                            &mut block_triggers,
                            &mut cur_obj_id,
                            &block.label,
                        );
                        let next_id = cur_obj_id + 1;
                        let ret_addr = next_return_addr(next_id);
                        pending_actions.push(make_vproc_set_dest(src_st, val_field_epd, Modifier::SetTo));
                        pending_actions.push(make_vproc_set_modifier(src_st, Modifier::SetTo));
                        pending_actions.push(make_vproc_set_nextptr(src_st, ret_addr));
                        let acts = std::mem::take(&mut pending_actions);
                        emit_custom_trigger(
                            format!("{}_write_ret_{}", block.label, cur_obj_id),
                            vproc_trigger_addr(src_st),
                            Vec::new(),
                            acts,
                            true,
                            &mut block_triggers,
                            &mut cur_obj_id,
                        );
                    }
                }
                ExpandedOp::FuncPtrAssignTarget { ptr, target } => {
                    ensure_capacity(
                        1,
                        &mut pending_actions,
                        &mut block_triggers,
                        &mut cur_obj_id,
                        &block.label,
                    );
                    let tramp_block = fp_trampoline_block[ptr];
                    let tramp_trig = block_first_trigger[&tramp_block];
                    let next_field_epd = ValueExpr::FieldEpd(FieldRef::new(tramp_trig, TriggerSlotField::NextPtr));
                    let target_entry_block = fn_entry_block[target];
                    let target_trig = block_first_trigger[&target_entry_block];
                    let target_addr = ValueExpr::ObjectAddress {
                        object: target_trig,
                        byte_offset: 0,
                    };
                    pending_actions.push(make_set_deaths_action(
                        next_field_epd,
                        Modifier::SetTo,
                        target_addr,
                        None,
                    ));
                }
            }
        }

        match &block.terminator {
            ExpandedTerminator::Jump(target_block) => {
                flush_pending_normal(
                    &mut pending_actions,
                    &mut block_triggers,
                    &mut cur_obj_id,
                    &block.label,
                );
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
                let target_obj = block_first_trigger[target_block];
                let last_trig = block_triggers.last_mut().unwrap();
                if matches!(last_trig.next_ptr, ValueExpr::ObjectAddress { object, .. } if object.0 == cur_obj_id)
                    || last_trig.next_ptr == ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                {
                    last_trig.next_ptr = ValueExpr::ObjectAddress {
                        object: target_obj,
                        byte_offset: 0,
                    };
                }
            }
            ExpandedTerminator::Return => {
                flush_pending_normal(
                    &mut pending_actions,
                    &mut block_triggers,
                    &mut cur_obj_id,
                    &block.label,
                );
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
                let last_trig = block_triggers.last_mut().unwrap();
                if matches!(last_trig.next_ptr, ValueExpr::ObjectAddress { object, .. } if object.0 == cur_obj_id)
                {
                    last_trig.next_ptr = ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32);
                }
            }
            ExpandedTerminator::FunctionReturn(fid) => {
                flush_pending_normal(
                    &mut pending_actions,
                    &mut block_triggers,
                    &mut cur_obj_id,
                    &block.label,
                );
                if block_triggers.is_empty() {
                    let id = ObjectId(cur_obj_id);
                    cur_obj_id += 1;
                    block_triggers.push(LirTrigger {
                        id,
                        label: Some(format!("{}_fret", block.label)),
                        prev_ptr: ValueExpr::Const(0),
                        next_ptr: ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
                        conditions: Vec::new(),
                        actions: Vec::new(),
                        flags: 4,
                    });
                }
                let ret_target_addr = if let Some(slots) = func_ret_slots.get(fid) {
                    if !slots.is_empty() {
                        let vbuf = vbuf_obj_id.unwrap();
                        ValueExpr::FieldAddress(FieldRef::vproc_slot(
                            vbuf.0,
                            slots[0],
                            VProcSlotField::TriggerBase,
                        ))
                    } else if let Some(&trig) = fn_void_ret_trig.get(fid) {
                        ValueExpr::ObjectAddress {
                            object: trig,
                            byte_offset: 0,
                        }
                    } else {
                        ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                    }
                } else if let Some(&trig) = fn_void_ret_trig.get(fid) {
                    ValueExpr::ObjectAddress {
                        object: trig,
                        byte_offset: 0,
                    }
                } else {
                    ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                };
                let last_trig = block_triggers.last_mut().unwrap();
                if matches!(last_trig.next_ptr, ValueExpr::ObjectAddress { object, .. } if object.0 == cur_obj_id)
                    || last_trig.next_ptr == ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                {
                    last_trig.next_ptr = ret_target_addr;
                }
            }
            ExpandedTerminator::Branch {
                conditions,
                then_block,
                else_block,
            } => {
                ensure_capacity(
                    1,
                    &mut pending_actions,
                    &mut block_triggers,
                    &mut cur_obj_id,
                    &block.label,
                );
                let branch_id = ObjectId(cur_obj_id + 1);
                let then_obj = block_first_trigger[then_block];
                let else_obj = block_first_trigger[else_block];
                let branch_next_field = FieldRef::new(branch_id, TriggerSlotField::NextPtr);

                pending_actions.push(LirAction {
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
                });
                flush_pending_normal(
                    &mut pending_actions,
                    &mut block_triggers,
                    &mut cur_obj_id,
                    &format!("{}_branch_reset", block.label),
                );

                debug_assert_eq!(ObjectId(cur_obj_id), branch_id);
                cur_obj_id += 1;

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

                block_triggers.push(branch_trig);
            }
        }

        // Resolve any placeholder `ObjectId(u32::MAX)` in `PatchNextTriggerCond` / `PatchNextTriggerAct`:
        // For any trigger i with a placeholder FieldRef, scan forward to the first subsequent trigger j > i
        // that is NOT a patch trigger (for Cond, a trigger with non-empty conditions; for Act, the next non-patch trigger).
        for i in 0..block_triggers.len() {
            let has_placeholder = block_triggers[i].actions.iter().any(|a| {
                matches!(
                    a.player2,
                    ValueExpr::FieldEpd(FieldRef {
                        object: ObjectId(u32::MAX),
                        ..
                    })
                )
            });
            if !has_placeholder {
                continue;
            }
            let is_cond_patch = block_triggers[i].actions.iter().any(|a| {
                matches!(
                    a.player2,
                    ValueExpr::FieldEpd(FieldRef {
                        object: ObjectId(u32::MAX),
                        field: TriggerSlotField::Condition(_, _),
                    })
                )
            });
            let target_id = block_triggers[i + 1..]
                .iter()
                .find(|t| {
                    let is_patch = t.label.as_deref().is_some_and(|l| {
                        l.contains("_patch_cond_") || l.contains("_patch_act_")
                    });
                    if is_patch {
                        return false;
                    }
                    if is_cond_patch {
                        !t.conditions.is_empty()
                    } else {
                        true
                    }
                })
                .map(|t| t.id)
                .expect("patch trigger must be followed by target trigger");

            for act in &mut block_triggers[i].actions {
                if let ValueExpr::FieldEpd(fr) = &mut act.player2 {
                    if fr.object == ObjectId(u32::MAX) {
                        fr.object = target_id;
                    }
                }
            }
        }

        for t in block_triggers {
            objects.push(LirObject::Trigger(t));
        }
    }

    for (fid, void_id) in &fn_void_ret_trig {
        objects.push(LirObject::Trigger(LirTrigger {
            id: *void_id,
            label: Some(format!("void_ret_{}", fid.0)),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32),
            conditions: Vec::new(),
            actions: Vec::new(),
            flags: 4,
        }));
    }

    debug_assert_eq!(cur_obj_id + fn_void_ret_trig.len() as u32, trigger_count);

    if let Some(vbuf_id) = vbuf_obj_id {
        objects.push(LirObject::VProcBuffer(LirVProcBuffer {
            id: vbuf_id,
            slots: vproc_slots,
        }));
    }

    for buf in &hir.byte_bufs {
        let obj_id = buf_id_to_obj[&buf.id];
        objects.push(LirObject::ByteBuf(LirByteBuf {
            id: obj_id,
            label: Some(buf.name.clone()),
            data: buf.data.clone(),
        }));
    }

    for (obj_id, var) in cell_vars {
        objects.push(LirObject::ByteBuf(LirByteBuf {
            id: obj_id,
            label: Some(format!("var_{}", var.name)),
            data: var.init_val.to_le_bytes().to_vec(),
        }));
    }

    let entry_trigger = block_first_trigger[&expanded_blocks[0].id];
    Ok(LirModule::new(&hir.name, objects, entry_trigger))
}

#[allow(clippy::too_many_arguments)]
fn emit_dynamic_div_chain(
    dividend: VarId,
    divisor: VarId,
    quot_dst: VarId,
    div_x_scratch: VarId,
    var_storage: &HashMap<VarId, VarStorage>,
    lower_cond: &dyn Fn(&StockCondition) -> LirCondition,
    block_triggers: &mut Vec<LirTrigger>,
    cur_obj_id: &mut u32,
    blk_label: &str,
    next_return_addr: &dyn Fn(u32) -> ValueExpr,
) {
    let divisor_st = var_storage[&divisor];
    let x_st = var_storage[&div_x_scratch];
    let x_epd = x_st.value_epd();
    let quot_epd = var_storage[&quot_dst].value_epd();
    let dividend_epd = var_storage[&dividend].value_epd();

    // Total triggers: 1 (init) + 32 * 5 (fill & double) + 32 (subtraction chain) = 193 triggers
    let chain_base_id = *cur_obj_id + 1 + 32 * 5;

    // 1. Init trigger: quot_dst = 0, and VProc(divisor -> div_x_scratch)
    let first_loop_id = ObjectId(*cur_obj_id + 1);
    let init_id = ObjectId(*cur_obj_id);
    *cur_obj_id += 1;
    block_triggers.push(LirTrigger {
        id: init_id,
        label: Some(format!("{blk_label}_div_init_{}", init_id.0)),
        prev_ptr: ValueExpr::Const(0),
        next_ptr: vproc_trigger_addr(divisor_st),
        conditions: Vec::new(),
        actions: vec![
            make_set_deaths_action(quot_epd, Modifier::SetTo, ValueExpr::Const(0), None),
            make_vproc_set_dest(divisor_st, x_epd, Modifier::SetTo),
            make_vproc_set_modifier(divisor_st, Modifier::SetTo),
            make_vproc_set_nextptr(
                divisor_st,
                ValueExpr::ObjectAddress {
                    object: first_loop_id,
                    byte_offset: 0,
                },
            ),
        ],
        flags: 4,
    });

    // 2. 32 iterations of 5 triggers each:
    //   t0: VProc(x -> chain[i].Cond[0].Amount)
    //   t1: VProc(x -> chain[i].Act[0].Player2)
    //   p1: if x & 0x80000000, jump to p3 else p2
    //   p3: restore p1.nextptr = p2, jump to chain[i]
    //   p2: VProc(x += x), jump to next iteration (or chain[31] on i == 31)
    for i in 0..32u32 {
        // Note: chain triggers are emitted in reverse order (31 down to 0) at chain_base_id..chain_base_id+32!
        // Specifically, chain_trig_for_i is at `chain_base_id + (31 - i)`.
        let chain_i_id = ObjectId(chain_base_id + (31 - i));
        let cx0_epd = ValueExpr::FieldEpd(FieldRef::condition(chain_i_id.0, 0, CondField::Amount));
        let cx1_epd = ValueExpr::FieldEpd(FieldRef::action(chain_i_id.0, 0, ActField::Player2));

        let t0_id = ObjectId(*cur_obj_id);
        let t1_id = ObjectId(*cur_obj_id + 1);
        let p1_id = ObjectId(*cur_obj_id + 2);
        let p3_id = ObjectId(*cur_obj_id + 3);
        let p2_id = ObjectId(*cur_obj_id + 4);
        let next_iter_id = ObjectId(*cur_obj_id + 5);
        *cur_obj_id += 5;

        // t0
        block_triggers.push(LirTrigger {
            id: t0_id,
            label: Some(format!("{blk_label}_div_fill0_{i}")),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: vproc_trigger_addr(x_st),
            conditions: Vec::new(),
            actions: vec![
                make_vproc_set_dest(x_st, cx0_epd, Modifier::SetTo),
                make_vproc_set_modifier(x_st, Modifier::SetTo),
                make_vproc_set_nextptr(
                    x_st,
                    ValueExpr::ObjectAddress {
                        object: t1_id,
                        byte_offset: 0,
                    },
                ),
            ],
            flags: 4,
        });
        // t1
        block_triggers.push(LirTrigger {
            id: t1_id,
            label: Some(format!("{blk_label}_div_fill1_{i}")),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: vproc_trigger_addr(x_st),
            conditions: Vec::new(),
            actions: vec![
                make_vproc_set_dest(x_st, cx1_epd, Modifier::SetTo),
                make_vproc_set_nextptr(
                    x_st,
                    ValueExpr::ObjectAddress {
                        object: p1_id,
                        byte_offset: 0,
                    },
                ),
            ],
            flags: 4,
        });
        // p1
        let p1_nptr_epd = ValueExpr::FieldEpd(FieldRef::new(p1_id, TriggerSlotField::NextPtr));
        block_triggers.push(LirTrigger {
            id: p1_id,
            label: Some(format!("{blk_label}_div_p1_{i}")),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: ValueExpr::ObjectAddress {
                object: p2_id,
                byte_offset: 0,
            },
            conditions: vec![lower_cond(&StockCondition::VarCmp {
                var: div_x_scratch,
                comparison: Comparison::AtLeast,
                value: 1,
                mask: Some(0x8000_0000),
            })],
            actions: vec![make_set_deaths_action(
                p1_nptr_epd,
                Modifier::SetTo,
                ValueExpr::ObjectAddress {
                    object: p3_id,
                    byte_offset: 0,
                },
                None,
            )],
            flags: 4,
        });
        // p3
        block_triggers.push(LirTrigger {
            id: p3_id,
            label: Some(format!("{blk_label}_div_p3_{i}")),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: ValueExpr::ObjectAddress {
                object: chain_i_id,
                byte_offset: 0,
            },
            conditions: Vec::new(),
            actions: vec![make_set_deaths_action(
                p1_nptr_epd,
                Modifier::SetTo,
                ValueExpr::ObjectAddress {
                    object: p2_id,
                    byte_offset: 0,
                },
                None,
            )],
            flags: 4,
        });
        // p2: x += x
        block_triggers.push(LirTrigger {
            id: p2_id,
            label: Some(format!("{blk_label}_div_p2_{i}")),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: vproc_trigger_addr(x_st),
            conditions: Vec::new(),
            actions: vec![
                make_vproc_set_dest(x_st, x_epd, Modifier::SetTo),
                make_vproc_set_modifier(x_st, Modifier::Add),
                make_vproc_set_nextptr(
                    x_st,
                    ValueExpr::ObjectAddress {
                        object: next_iter_id,
                        byte_offset: 0,
                    },
                ),
            ],
            flags: 4,
        });
    }

    debug_assert_eq!(*cur_obj_id, chain_base_id);

    // 3. 32 subtraction chain triggers (i = 31 down to 0)
    for i in (0..32u32).rev() {
        let cid = ObjectId(*cur_obj_id);
        *cur_obj_id += 1;
        let next_addr = if i == 0 {
            next_return_addr(*cur_obj_id)
        } else {
            ValueExpr::ObjectAddress {
                object: ObjectId(*cur_obj_id),
                byte_offset: 0,
            }
        };
        let mut acts = vec![
            make_set_deaths_action(dividend_epd, Modifier::Subtract, ValueExpr::Const(0), None),
            make_set_deaths_action(
                quot_epd,
                Modifier::Add,
                ValueExpr::Const((1u32 << i) as i32),
                None,
            ),
        ];
        if i == 0 {
            acts.push(make_vproc_set_modifier(x_st, Modifier::SetTo));
        }
        block_triggers.push(LirTrigger {
            id: cid,
            label: Some(format!("{blk_label}_div_chain_{i}")),
            prev_ptr: ValueExpr::Const(0),
            next_ptr: next_addr,
            conditions: vec![lower_cond(&StockCondition::VarCmp {
                var: dividend,
                comparison: Comparison::AtLeast,
                value: 0,
                mask: None,
            })],
            actions: acts,
            flags: 4,
        });
    }
}
