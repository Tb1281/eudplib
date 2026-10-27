use crate::error::{EudkitError, Result};
use crate::hir::{
    epd, BlockId, ByteBufId, HirBasicBlock, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, Modifier, StockAction, StockCondition, VarId, END_OF_TRIGGERS_SENTINEL,
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

    pub fn vproc_count(&self) -> usize {
        0
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

fn stmt_to_leaf_action(stmt: &HirStmt) -> Option<StockAction> {
    match stmt {
        HirStmt::Action(a) => Some(a.clone()),
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
        | StockAction::SetMemory { .. }
        | StockAction::SetMemoryX { .. }
        | StockAction::SetField { .. } => false,
    })
}

fn flatten_structured_cfg(func: &HirFunction) -> Result<HirFunction> {
    let has_structured_cf = func.blocks.iter().any(|b| {
        b.stmts.iter().any(|s| {
            matches!(
                s,
                HirStmt::If { .. }
                    | HirStmt::While { .. }
                    | HirStmt::Break
                    | HirStmt::Continue
            )
        })
    });

    if !has_structured_cf {
        let mut cloned = func.clone();
        for blk in &mut cloned.blocks {
            for stmt in &mut blk.stmts {
                if let Some(act) = stmt_to_leaf_action(stmt) {
                    *stmt = HirStmt::Action(act);
                }
            }
        }
        return Ok(cloned);
    }

    let mut out_blocks: Vec<HirBasicBlock> = func
        .blocks
        .iter()
        .map(|b| HirBasicBlock {
            id: b.id,
            label: b.label.clone(),
            stmts: Vec::new(),
            terminator: HirTerminator::Return,
        })
        .collect();

    fn alloc_block(out_blocks: &mut Vec<HirBasicBlock>, label: impl Into<String>) -> BlockId {
        let id = BlockId(out_blocks.len() as u32);
        let mut blk = HirBasicBlock::new(label);
        blk.id = id;
        out_blocks.push(blk);
        id
    }

    fn lower_stmts(
        stmts: &[HirStmt],
        mut cur_block: BlockId,
        out_blocks: &mut Vec<HirBasicBlock>,
        loop_ctx: Option<(BlockId, BlockId)>,
    ) -> Result<Option<BlockId>> {
        for stmt in stmts {
            if let Some(act) = stmt_to_leaf_action(stmt) {
                out_blocks[cur_block.0 as usize]
                    .stmts
                    .push(HirStmt::Action(act));
                continue;
            }
            match stmt {
                HirStmt::Trigger { .. } => {
                    out_blocks[cur_block.0 as usize].stmts.push(stmt.clone());
                }
                HirStmt::Action(_)
                | HirStmt::VarAssignConst { .. }
                | HirStmt::VarAddConst { .. }
                | HirStmt::VarSubConst { .. }
                | HirStmt::VarSetMaskConst { .. } => unreachable!(),
                HirStmt::Break => {
                    let (_, break_target) = loop_ctx.ok_or_else(|| {
                        EudkitError::Ir("break statement outside of while loop".to_string())
                    })?;
                    out_blocks[cur_block.0 as usize]
                        .set_terminator(HirTerminator::Jump(break_target));
                    return Ok(None);
                }
                HirStmt::Continue => {
                    let (cond_target, _) = loop_ctx.ok_or_else(|| {
                        EudkitError::Ir("continue statement outside of while loop".to_string())
                    })?;
                    out_blocks[cur_block.0 as usize]
                        .set_terminator(HirTerminator::Jump(cond_target));
                    return Ok(None);
                }
                HirStmt::If {
                    conditions,
                    then_body,
                    else_body,
                } => {
                    if else_body.is_empty() && conditions.len() <= 16 {
                        if let Some(leaf_actions) = extract_leaf_actions(then_body) {
                            if leaf_actions.is_empty() {
                                continue;
                            }
                            if leaf_actions.len() <= 64 {
                                out_blocks[cur_block.0 as usize].stmts.push(HirStmt::Trigger {
                                    label: Some("if_then".to_string()),
                                    conditions: conditions.clone(),
                                    actions: leaf_actions,
                                    preserved: true,
                                });
                                continue;
                            }
                        }
                    } else if conditions.len() == 1 {
                        if let Some(inv_cond) = conditions[0].invert() {
                            if let (Some(then_actions), Some(else_actions)) = (
                                extract_leaf_actions(then_body),
                                extract_leaf_actions(else_body),
                            ) {
                                if then_actions.len() <= 64
                                    && else_actions.len() <= 64
                                    && actions_cannot_mutate_condition(
                                        &then_actions,
                                        &conditions[0],
                                    )
                                {
                                    if !then_actions.is_empty() {
                                        out_blocks[cur_block.0 as usize].stmts.push(
                                            HirStmt::Trigger {
                                                label: Some("if_then_inplace".to_string()),
                                                conditions: conditions.clone(),
                                                actions: then_actions,
                                                preserved: true,
                                            },
                                        );
                                    }
                                    if !else_actions.is_empty() {
                                        out_blocks[cur_block.0 as usize].stmts.push(
                                            HirStmt::Trigger {
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
                        out_blocks[cur_block.0 as usize].set_terminator(HirTerminator::Branch {
                            conditions: conditions.clone(),
                            then_block,
                            else_block: merge_block,
                        });
                        if let Some(then_end) =
                            lower_stmts(then_body, then_block, out_blocks, loop_ctx)?
                        {
                            out_blocks[then_end.0 as usize]
                                .set_terminator(HirTerminator::Jump(merge_block));
                        }
                        cur_block = merge_block;
                    } else {
                        let else_block = alloc_block(out_blocks, "if_else");
                        out_blocks[cur_block.0 as usize].set_terminator(HirTerminator::Branch {
                            conditions: conditions.clone(),
                            then_block,
                            else_block,
                        });
                        let then_end = lower_stmts(then_body, then_block, out_blocks, loop_ctx)?;
                        let else_end = lower_stmts(else_body, else_block, out_blocks, loop_ctx)?;
                        match (then_end, else_end) {
                            (None, None) => return Ok(None),
                            _ => {
                                let merge_block = alloc_block(out_blocks, "if_merge");
                                if let Some(te) = then_end {
                                    out_blocks[te.0 as usize]
                                        .set_terminator(HirTerminator::Jump(merge_block));
                                }
                                if let Some(ee) = else_end {
                                    out_blocks[ee.0 as usize]
                                        .set_terminator(HirTerminator::Jump(merge_block));
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

                    out_blocks[cur_block.0 as usize]
                        .set_terminator(HirTerminator::Jump(cond_block));
                    out_blocks[cond_block.0 as usize].set_terminator(HirTerminator::Branch {
                        conditions: conditions.clone(),
                        then_block: body_block,
                        else_block: exit_block,
                    });

                    if let Some(body_end) =
                        lower_stmts(body, body_block, out_blocks, Some((cond_block, exit_block)))?
                    {
                        out_blocks[body_end.0 as usize]
                            .set_terminator(HirTerminator::Jump(cond_block));
                    }
                    cur_block = exit_block;
                }
            }
        }
        Ok(Some(cur_block))
    }

    for orig_block in &func.blocks {
        if let Some(end_block) =
            lower_stmts(&orig_block.stmts, orig_block.id, &mut out_blocks, None)?
        {
            out_blocks[end_block.0 as usize].set_terminator(orig_block.terminator.clone());
        }
    }

    // Forward empty pass-through Jump blocks (except entry block 0) so Break/Continue/Merge
    // don't emit redundant no-op pass-through triggers.
    let resolve_jump = |mut bid: BlockId, blocks: &[HirBasicBlock]| -> BlockId {
        for _ in 0..64 {
            if bid.0 == 0 {
                break;
            }
            let blk = &blocks[bid.0 as usize];
            if blk.stmts.is_empty() {
                if let HirTerminator::Jump(next) = blk.terminator {
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
        match &mut blk.terminator {
            HirTerminator::Jump(target) => {
                *target = resolve_jump(*target, &snapshot);
            }
            HirTerminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                *then_block = resolve_jump(*then_block, &snapshot);
                *else_block = resolve_jump(*else_block, &snapshot);
            }
            HirTerminator::Return => {}
        }
    }

    // Collect reachable blocks starting from BlockId(0)
    let mut reachable = vec![false; out_blocks.len()];
    let mut worklist = vec![BlockId(0)];
    reachable[0] = true;
    while let Some(bid) = worklist.pop() {
        let blk = &out_blocks[bid.0 as usize];
        match &blk.terminator {
            HirTerminator::Jump(t) => {
                if !reachable[t.0 as usize] {
                    reachable[t.0 as usize] = true;
                    worklist.push(*t);
                }
            }
            HirTerminator::Branch {
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
            HirTerminator::Return => {}
        }
    }

    let mut old_to_new: HashMap<BlockId, BlockId> = HashMap::new();
    let mut compact_blocks: Vec<HirBasicBlock> = Vec::new();
    for (idx, mut blk) in out_blocks.into_iter().enumerate() {
        if reachable[idx] {
            let new_id = BlockId(compact_blocks.len() as u32);
            old_to_new.insert(blk.id, new_id);
            blk.id = new_id;
            compact_blocks.push(blk);
        }
    }
    for blk in &mut compact_blocks {
        match &mut blk.terminator {
            HirTerminator::Jump(t) => *t = old_to_new[t],
            HirTerminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                *then_block = old_to_new[then_block];
                *else_block = old_to_new[else_block];
            }
            HirTerminator::Return => {}
        }
    }

    Ok(HirFunction {
        id: func.id,
        name: func.name.clone(),
        is_main_loop: func.is_main_loop,
        blocks: compact_blocks,
    })
}

pub fn lower_hir_to_lir(hir: &HirModule, map_ctx: &mut MapContext) -> Result<LirModule> {
    let main_fn_id = hir
        .main_loop
        .or_else(|| hir.functions.first().map(|f| f.id))
        .ok_or_else(|| EudkitError::Ir("HIR module has no functions".to_string()))?;
    let raw_main_fn = &hir.functions[main_fn_id.0 as usize];
    if raw_main_fn.blocks.is_empty() {
        return Err(EudkitError::Ir(format!(
            "function '{}' has no basic blocks",
            raw_main_fn.name
        )));
    }
    let main_fn = flatten_structured_cfg(raw_main_fn)?;

    // Reserve ObjectIds for triggers first so Trigger 0 gets @obj0 (predictable FieldRef IDs),
    // and allocate ByteBufs (including 4-byte lowered variables) right after triggers.
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
                _ => unreachable!("flatten_structured_cfg must lower all structured HirStmts"),
            }
        }
        if matches!(block.terminator, HirTerminator::Branch { .. }) {
            // Coalesce the branch_reset action into pending_actions if < 64 actions are pending,
            // otherwise emit a dedicated reset trigger before the branch condition trigger.
            pending_actions += 1;
            if pending_actions == 64 {
                block_trigs += 1;
                pending_actions = 0;
            }
            if pending_actions > 0 {
                block_trigs += 1;
            }
            block_trigs += 1; // branch_cond trigger
        } else {
            if pending_actions > 0 {
                block_trigs += 1;
            }
            if block_trigs == 0 {
                // Empty Jump/Return block needs 1 pass-through trigger
                block_trigs += 1;
            }
        }
        trigger_count += block_trigs;
    }

    let mut buf_id_to_obj: HashMap<ByteBufId, ObjectId> = HashMap::new();
    for (idx, buf) in hir.byte_bufs.iter().enumerate() {
        buf_id_to_obj.insert(buf.id, ObjectId(trigger_count + idx as u32));
    }

    let var_base_obj_idx = trigger_count + hir.byte_bufs.len() as u32;
    let mut var_id_to_obj: HashMap<VarId, ObjectId> = HashMap::new();
    for (idx, var) in hir.vars.iter().enumerate() {
        var_id_to_obj.insert(var.id, ObjectId(var_base_obj_idx + idx as u32));
    }

    let lower_val = |v: HirValue,
                     buf_map: &HashMap<ByteBufId, ObjectId>,
                     var_map: &HashMap<VarId, ObjectId>|
     -> ValueExpr {
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
            HirValue::VarAddress(vid) => ValueExpr::ObjectAddress {
                object: var_map[&vid],
                byte_offset: 0,
            },
            HirValue::VarEpd(vid) => ValueExpr::FieldEpd(FieldRef::new(
                var_map[&vid],
                TriggerSlotField::ByteOffset(0),
            )),
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
                let var_obj = var_id_to_obj[&var];
                let var_epd = ValueExpr::FieldEpd(FieldRef::new(
                    var_obj,
                    TriggerSlotField::ByteOffset(0),
                ));
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
                player2: lower_val(*value, &buf_id_to_obj, &var_id_to_obj),
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
                player2: lower_val(*value, &buf_id_to_obj, &var_id_to_obj),
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
                player2: lower_val(*value, &buf_id_to_obj, &var_id_to_obj),
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
                player2: lower_val(*value, &buf_id_to_obj, &var_id_to_obj),
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
                let var_obj = var_id_to_obj[var];
                let var_epd = ValueExpr::FieldEpd(FieldRef::new(
                    var_obj,
                    TriggerSlotField::ByteOffset(0),
                ));
                match mask {
                    None => LirAction {
                        locid1: ValueExpr::Const(0),
                        strid: ValueExpr::Const(0),
                        wavid: ValueExpr::Const(0),
                        time: ValueExpr::Const(0),
                        player1: var_epd,
                        player2: ValueExpr::Const(*value),
                        unitid: 0,
                        acttype: 45,
                        amount: modifier.encode(),
                        flags: 20,
                        eudx: 0,
                    },
                    Some(m) => LirAction {
                        locid1: ValueExpr::Const(*m as i32),
                        strid: ValueExpr::Const(0),
                        wavid: ValueExpr::Const(0),
                        time: ValueExpr::Const(0),
                        player1: var_epd,
                        player2: ValueExpr::Const(*value),
                        unitid: 0,
                        acttype: 45,
                        amount: modifier.encode(),
                        flags: 20,
                        eudx: EUDX_SIGNATURE,
                    },
                }
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
                player2: lower_val(*value, &buf_id_to_obj, &var_id_to_obj),
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

    let mut objects: Vec<LirObject> =
        Vec::with_capacity(trigger_count as usize + hir.byte_bufs.len() + hir.vars.len());
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
                _ => unreachable!("flatten_structured_cfg must lower all structured HirStmts"),
            }
        }

        match &block.terminator {
            HirTerminator::Jump(target_block) => {
                flush_pending(
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
                flush_pending(
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
                // Pre-calculate branch_id: flush_pending will allocate 1 trigger (holding
                // any remaining pending_actions + the branch_reset action), followed by branch_id!
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
                flush_pending(
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

    for var in &hir.vars {
        let obj_id = var_id_to_obj[&var.id];
        objects.push(LirObject::ByteBuf(LirByteBuf {
            id: obj_id,
            label: Some(format!("var_{}", var.name)),
            data: var.init_val.to_le_bytes().to_vec(),
        }));
    }

    let entry_trigger = block_first_trigger[&main_fn.blocks[0].id];
    Ok(LirModule::new(&hir.name, objects, entry_trigger))
}
