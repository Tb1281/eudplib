//! One-shot planning of object layout before trigger emission.
//!
//! `emit_lir` used to interleave three concerns: counting triggers per block,
//! deciding where every non-trigger object lives, and assigning VProc slots.
//! `EmissionPlan` owns that layout. Emission consumes the plan, so block
//! reservations, byte buffers, VProc slots, void-return triggers, and the read
//! table all get their `ObjectId` from one place before the first trigger is
//! written.

use crate::hir::{BlockId, ByteBufId, FuncId, FuncPtrId, HirModule, Modifier, VarId};
use crate::lir::read_table::ReadTable;
use crate::lir::storage_selection::{collect_vproc_vars, VarStorage};
use crate::lir::trigger_shape::{block_trigger_shape, OpTriggerSpan, TerminatorTriggerSpan};
use crate::lir::types::*;
use std::collections::{HashMap, HashSet};

/// The trigger address space reserved for one expanded block.
///
/// `first_trigger` and `trigger_budget` come from [`block_trigger_shape`], the
/// same policy `emit_lir` consumes for its per-op spans, so a block can verify
/// it emitted exactly the planned span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPlan {
    pub id: BlockId,
    pub first_trigger: ObjectId,
    pub trigger_budget: u32,
    /// Number of `PatchNextTrigger*` ops in the block, checked after emission.
    pub patch_sites: u32,
    op_spans: Vec<OpTriggerSpan>,
    terminator_span: TerminatorTriggerSpan,
}

impl BlockPlan {
    pub fn end_trigger(&self) -> ObjectId {
        ObjectId(self.first_trigger.0 + self.trigger_budget)
    }

    /// Per-[`ExpandedOp`] trigger spans, in block order.
    pub fn op_spans(&self) -> &[OpTriggerSpan] {
        &self.op_spans
    }

    /// Trigger span owned by the op at `index`.
    pub fn op_span(&self, index: usize) -> &OpTriggerSpan {
        &self.op_spans[index]
    }

    /// Trigger span owned by the block terminator.
    pub fn terminator_span(&self) -> &TerminatorTriggerSpan {
        &self.terminator_span
    }
}

/// Objects that emission appends after the block triggers.
///
/// They are already addressed and named by the plan; emission only moves them
/// into the `LirModule` in the planned order.
#[derive(Debug)]
pub struct PlannedObjects {
    pub void_return_triggers: HashMap<FuncId, ObjectId>,
    pub read_table_triggers: Vec<LirTrigger>,
    pub vproc_buffer: Option<LirVProcBuffer>,
    pub byte_bufs: Vec<LirByteBuf>,
    pub cell_bufs: Vec<LirByteBuf>,
    pub entry_trigger: ObjectId,
}

/// The complete object layout for one `HirModule`.
///
/// Build it once with [`EmissionPlan::build`], then ask it for block spans,
/// variable storage, return slots, and the planned objects at the end.
#[derive(Debug)]
pub struct EmissionPlan {
    blocks: Vec<BlockPlan>,
    first_trigger_by_block: HashMap<BlockId, ObjectId>,
    void_return_triggers: HashMap<FuncId, ObjectId>,
    read_table: Option<ReadTable>,
    block_trigger_end: u32,
    trigger_count: u32,
    next_object_id: u32,
    var_storage: HashMap<VarId, VarStorage>,
    byte_buf_ids: HashMap<ByteBufId, ObjectId>,
    planned_objects: PlannedObjects,
    func_return_slots: HashMap<FuncId, Vec<u32>>,
    func_ptr_return_slots: HashMap<FuncPtrId, Vec<u32>>,
}

impl EmissionPlan {
    /// Reserves every `ObjectId` the module will need.
    ///
    /// The order is observable and must stay stable: block triggers, void-return
    /// triggers, the read table, the VProc buffer, HIR byte buffers, then
    /// non-promoted variable cells.
    pub fn build(hir: &HirModule, cfg: &ExpandedCfg, main_fn_id: FuncId) -> Self {
        let vproc_var_set = collect_vproc_vars(&cfg.blocks);

        let mut blocks = Vec::with_capacity(cfg.blocks.len());
        let mut first_trigger_by_block = HashMap::new();
        let mut block_trigger_end = 0u32;
        for block in &cfg.blocks {
            let shape = block_trigger_shape(block);
            let first_trigger = ObjectId(block_trigger_end);
            first_trigger_by_block.insert(block.id, first_trigger);
            blocks.push(BlockPlan {
                id: block.id,
                first_trigger,
                trigger_budget: shape.trigger_count,
                patch_sites: shape.patch_sites,
                op_spans: shape.ops,
                terminator_span: shape.terminator,
            });
            block_trigger_end += shape.trigger_count;
        }

        let mut trigger_count = block_trigger_end;
        let mut void_return_triggers = HashMap::new();
        for func in &hir.functions {
            if func.id == main_fn_id || !cfg.fn_entry_block.contains_key(&func.id) {
                continue;
            }
            if func.return_count == 0 {
                void_return_triggers.insert(func.id, ObjectId(trigger_count));
                trigger_count += 1;
            }
        }

        let has_mem_read = cfg.blocks.iter().any(|block| {
            block
                .ops
                .iter()
                .any(|op| matches!(op, ExpandedOp::MemReadSetup { .. }))
        });
        let read_table = if has_mem_read {
            let table = ReadTable::build(trigger_count);
            trigger_count += table.triggers.len() as u32;
            Some(table)
        } else {
            None
        };

        let has_ret_slots = hir
            .functions
            .iter()
            .any(|f| f.return_count > 0 && cfg.fn_entry_block.contains_key(&f.id))
            || hir.func_ptrs.iter().any(|fp| fp.signature.1 > 0);

        let mut next_object_id = trigger_count;
        let vbuf_obj_id = if !vproc_var_set.is_empty() || has_ret_slots {
            let id = ObjectId(next_object_id);
            next_object_id += 1;
            Some(id)
        } else {
            None
        };

        let mut byte_buf_ids = HashMap::new();
        let mut byte_bufs = Vec::with_capacity(hir.byte_bufs.len());
        for buf in &hir.byte_bufs {
            let id = ObjectId(next_object_id);
            next_object_id += 1;
            byte_buf_ids.insert(buf.id, id);
            byte_bufs.push(LirByteBuf {
                id,
                label: Some(buf.name.clone()),
                data: buf.data.clone(),
            });
        }

        let mut var_storage = HashMap::new();
        let mut vproc_slots: Vec<LirVProcSlot> = Vec::new();
        let mut cell_bufs = Vec::new();
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
                let id = ObjectId(next_object_id);
                next_object_id += 1;
                var_storage.insert(var.id, VarStorage::ByteBuf(id));
                cell_bufs.push(LirByteBuf {
                    id,
                    label: Some(format!("var_{}", var.name)),
                    data: var.init_val.to_le_bytes().to_vec(),
                });
            }
        }

        let mut func_return_slots = HashMap::new();
        let mut func_ptr_return_slots = HashMap::new();
        if let Some(vbuf) = vbuf_obj_id {
            // FuncPtr targets share one set of return slots.
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
                    link_return_slots(&mut vproc_slots, vbuf, &slots);
                    func_ptr_return_slots.insert(fp.id, slots.clone());

                    let mut fp_targets = HashSet::new();
                    if let Some(target) = fp.initial_target {
                        fp_targets.insert(target);
                    }
                    for block in &cfg.blocks {
                        for op in &block.ops {
                            if let ExpandedOp::FuncPtrAssignTarget { ptr, target } = op {
                                if *ptr == fp.id {
                                    fp_targets.insert(*target);
                                }
                            }
                        }
                    }
                    for target in fp_targets {
                        func_return_slots.insert(target, slots.clone());
                    }
                }
            }

            // Remaining non-FuncPtr functions get their own return slots.
            for func in &hir.functions {
                if func.id == main_fn_id
                    || !cfg.fn_entry_block.contains_key(&func.id)
                    || func_return_slots.contains_key(&func.id)
                {
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
                    link_return_slots(&mut vproc_slots, vbuf, &slots);
                    func_return_slots.insert(func.id, slots);
                }
            }
        }

        let vproc_buffer = vbuf_obj_id.map(|id| LirVProcBuffer {
            id,
            slots: vproc_slots,
        });
        let entry_trigger = blocks
            .first()
            .map(|block| block.first_trigger)
            .expect("expanded CFG has no blocks");

        let planned_objects = PlannedObjects {
            void_return_triggers: void_return_triggers.clone(),
            read_table_triggers: read_table
                .as_ref()
                .map(|table| table.triggers.clone())
                .unwrap_or_default(),
            vproc_buffer,
            byte_bufs,
            cell_bufs,
            entry_trigger,
        };

        Self {
            blocks,
            first_trigger_by_block,
            void_return_triggers,
            read_table,
            block_trigger_end,
            trigger_count,
            next_object_id,
            var_storage,
            byte_buf_ids,
            planned_objects,
            func_return_slots,
            func_ptr_return_slots,
        }
    }

    pub fn blocks(&self) -> &[BlockPlan] {
        &self.blocks
    }

    pub fn block(&self, index: usize) -> &BlockPlan {
        &self.blocks[index]
    }

    pub fn first_trigger(&self, block: BlockId) -> ObjectId {
        self.first_trigger_by_block[&block]
    }

    /// One past the last block trigger; void-return triggers start here.
    pub fn block_trigger_end(&self) -> u32 {
        self.block_trigger_end
    }

    /// Total triggers, including void-return triggers and the read table.
    pub fn trigger_count(&self) -> u32 {
        self.trigger_count
    }

    /// One past every planned object.
    pub fn next_object_id(&self) -> u32 {
        self.next_object_id
    }

    pub fn var_storage(&self) -> &HashMap<VarId, VarStorage> {
        &self.var_storage
    }

    pub fn byte_buf_ids(&self) -> &HashMap<ByteBufId, ObjectId> {
        &self.byte_buf_ids
    }

    pub fn func_return_slots(&self, func: FuncId) -> Option<&[u32]> {
        self.func_return_slots.get(&func).map(Vec::as_slice)
    }

    pub fn func_ptr_return_slots(&self, ptr: FuncPtrId) -> Option<&[u32]> {
        self.func_ptr_return_slots.get(&ptr).map(Vec::as_slice)
    }

    pub fn void_return_trigger(&self, func: FuncId) -> Option<ObjectId> {
        self.void_return_triggers.get(&func).copied()
    }

    pub fn void_return_triggers(&self) -> &HashMap<FuncId, ObjectId> {
        &self.void_return_triggers
    }

    pub fn read_table(&self) -> Option<&ReadTable> {
        self.read_table.as_ref()
    }

    pub fn vproc_buffer(&self) -> Option<&LirVProcBuffer> {
        self.planned_objects.vproc_buffer.as_ref()
    }

    /// Consumes the plan and returns the objects emission appends verbatim.
    pub fn into_planned_objects(self) -> PlannedObjects {
        self.planned_objects
    }
}

fn link_return_slots(vproc_slots: &mut [LirVProcSlot], vbuf: ObjectId, slots: &[u32]) {
    for i in 0..slots.len().saturating_sub(1) {
        let next_slot = slots[i + 1];
        vproc_slots[slots[i] as usize].next_ptr = ValueExpr::FieldAddress(FieldRef::vproc_slot(
            vbuf.0,
            next_slot,
            VProcSlotField::TriggerBase,
        ));
    }
    vproc_slots[*slots.last().unwrap() as usize].next_ptr =
        ValueExpr::Const(crate::hir::END_OF_TRIGGERS_SENTINEL as i32);
}
