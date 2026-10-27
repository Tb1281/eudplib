use crate::cp_string::*;
use crate::error::{EudkitError, Result};
use crate::hir::{
    epd, BlockId, ByteBufId, Comparison, HirExpr, HirModule, HirValue, Modifier, StockAction,
    StockCondition, VarId, CURRENT_PLAYER_ADDR, END_OF_TRIGGERS_SENTINEL,
};
use crate::lir::read_table::ReadTable;
use crate::lir::storage_selection::{collect_vproc_vars, VarStorage};
use crate::lir::types::*;
use crate::map_context::MapContext;
use std::collections::{HashMap, HashSet};

fn count_leaf_action_actions(act: &StockAction) -> usize {
    match act {
        StockAction::PrintAt { parts, .. } => {
            crate::cp_string::emit_format_parts_actions(
                ValueExpr::Const(0),
                parts,
                None,
                false,
                &std::collections::HashMap::new(),
            ).len()
        }
        StockAction::FormatPrint { target, parts, always_display } => {
            let has_disp = matches!(target, crate::hir::PrintTarget::DisplayText);
            crate::cp_string::emit_format_parts_actions(
                ValueExpr::Const(0),
                parts,
                if has_disp { Some(1) } else { None },
                *always_display,
                &std::collections::HashMap::new(),
            ).len()
        }
        _ => 1,
    }
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
            ExpandedOp::LeafAction(act) => {
                let n = count_leaf_action_actions(act);
                for _ in 0..n {
                    add_pending(1, &mut pending_actions, &mut count);
                }
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
            ExpandedOp::MemReadSetup { epd, initial_val, .. } => {
                let acts = if *initial_val != 0 { 3 } else { 2 };
                match epd {
                    HirExpr::Const(_) => {
                        add_pending(acts + 1, &mut pending_actions, &mut count);
                    }
                    HirExpr::Var(_) => {
                        add_pending(acts + 3, &mut pending_actions, &mut count);
                        flush_pending(&mut pending_actions, &mut count);
                    }
                    _ => unreachable!(),
                }
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


pub fn emit_lir(
    hir: &HirModule,
    expanded_cfg: ExpandedCfg,
    map_ctx: &mut MapContext,
) -> Result<LirModule> {
    let main_fn_id = hir
        .main_loop
        .or_else(|| hir.functions.first().map(|f| f.id))
        .ok_or_else(|| EudkitError::Ir("HIR module has no functions".to_string()))?;
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

    let has_mem_read = expanded_blocks.iter().any(|b| {
        b.ops
            .iter()
            .any(|op| matches!(op, ExpandedOp::MemReadSetup { .. }))
    });
    let (read_table, read_table_count) = if has_mem_read {
        let rt = ReadTable::build(trigger_count);
        let count = rt.triggers.len() as u32;
        (Some(rt), count)
    } else {
        (None, 0)
    };
    trigger_count += read_table_count;

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
            StockCondition::ElapsedTime {
                comparison,
                seconds,
            } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(0),
                amount: ValueExpr::Const(seconds as i32),
                unitid: 0,
                comparison: comparison.encode(),
                condtype: 12,
                restype: 0,
                flags: 0,
                eudx: 0,
            },
            StockCondition::Score {
                player,
                score_type,
                comparison,
                amount,
            } => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(player.encode()),
                amount: ValueExpr::Const(amount as i32),
                unitid: 0,
                comparison: comparison.encode(),
                condtype: 21,
                restype: score_type,
                flags: 0,
                eudx: 0,
            },
            StockCondition::IsUserCp => LirCondition {
                locid: ValueExpr::Const(0),
                player: ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)),
                amount: ValueExpr::Const(0),
                unitid: 0,
                comparison: 10, // Exactly
                condtype: 15, // Deaths
                restype: 0,
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

    let lower_single_act = |a: &StockAction, map_ctx: &mut MapContext| -> LirAction {
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
            StockAction::KillUnitAt {
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
                acttype: 23,
                amount: *count,
                flags: 20,
                eudx: 0,
            },
            StockAction::MinimapPing { location } => LirAction {
                locid1: ValueExpr::Const(*location as i32),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(0),
                player2: ValueExpr::Const(0),
                unitid: 0,
                acttype: 28,
                amount: 0,
                flags: 4,
                eudx: 0,
            },
            StockAction::SetScore {
                player,
                modifier,
                amount,
                score_type,
            } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(player.encode()),
                player2: ValueExpr::Const(*amount as i32),
                unitid: *score_type as u16,
                acttype: 27,
                amount: modifier.encode(),
                flags: 4,
                eudx: 0,
            },
            StockAction::SetSwitch { switch, action } => LirAction {
                locid1: ValueExpr::Const(0),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(0),
                player2: ValueExpr::Const(*switch as i32),
                unitid: 0,
                acttype: 13,
                amount: action.encode(),
                flags: 4,
                eudx: 0,
            },
            StockAction::MoveLocation {
                location,
                unit,
                player,
                dest_location,
            } => LirAction {
                locid1: ValueExpr::Const(*dest_location as i32),
                strid: ValueExpr::Const(0),
                wavid: ValueExpr::Const(0),
                time: ValueExpr::Const(0),
                player1: ValueExpr::Const(player.encode()),
                player2: ValueExpr::Const(*location as i32),
                unitid: *unit,
                acttype: 38,
                amount: 0,
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
            StockAction::FormatPrint { .. } | StockAction::PrintAt { .. } => {
                unreachable!("lower_act must handle FormatPrint and PrintAt")
            }
        }
    };

    let var_inits: HashMap<VarId, i32> = hir.vars.iter().map(|v| (v.id, v.init_val)).collect();
    let lower_act = |a: &StockAction, map_ctx: &mut MapContext| -> Vec<LirAction> {
        match a {
            StockAction::PrintAt { line, parts } => {
                let dest_epd = ValueExpr::Const(epd(0x640B60 + 218 * (*line as u32)));
                emit_format_parts_actions(dest_epd, parts, None, false, &var_inits)
            }
            StockAction::FormatPrint { target, parts, always_display } => {
                let (dest_epd, display_str_id) = match target {
                    crate::hir::PrintTarget::ScreenLine(line) => {
                        (ValueExpr::Const(epd(0x640B60 + 218 * (*line as u32))), None)
                    }
                    crate::hir::PrintTarget::MapString(str_id) => {
                        let off = map_ctx.string_table().string_offset(*str_id).unwrap_or(0);
                        (ValueExpr::Const(epd(crate::hir::STRX_BASE_ADDR + off)), None)
                    }
                    crate::hir::PrintTarget::DisplayText => {
                        let buf_text = "\r\n                                                                                                                        \0";
                        let str_id = map_ctx.get_or_insert_string(buf_text.as_bytes());
                        let off = map_ctx.string_table().string_offset(str_id).unwrap_or(0);
                        (ValueExpr::Const(epd(crate::hir::STRX_BASE_ADDR + off)), Some(str_id))
                    }
                    crate::hir::PrintTarget::Memory(epd_expr) => {
                        let target_val = match epd_expr.fold_constants() {
                            crate::hir::HirExpr::Const(c) => ValueExpr::Const(c),
                            crate::hir::HirExpr::Var(v) => var_storage[&v].value_epd(),
                            _ => ValueExpr::Const(0),
                        };
                        (target_val, None)
                    }
                };
                emit_format_parts_actions(dest_epd, parts, display_str_id, *always_display, &var_inits)
            }
            other => vec![lower_single_act(other, map_ctx)],
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
            ExpandedTerminator::ReadTableJump { mask, shift } => {
                let rt = read_table.as_ref().expect("read_table must be present");
                ValueExpr::ObjectAddress {
                    object: rt.entry(*mask, *shift),
                    byte_offset: 0,
                }
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
                    let acts = lower_act(act, map_ctx);
                    for la in acts {
                        ensure_capacity(
                            1,
                            &mut pending_actions,
                            &mut block_triggers,
                            &mut cur_obj_id,
                            &block.label,
                        );
                        pending_actions.push(la);
                        if pending_actions.len() == 64 {
                            flush_pending_normal(
                                &mut pending_actions,
                                &mut block_triggers,
                                &mut cur_obj_id,
                                &block.label,
                            );
                        }
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
                    let mut lowered_actions = Vec::new();
                    for a in actions {
                        lowered_actions.extend(lower_act(a, map_ctx));
                    }
                    if conditions.len() > 16 {
                        return Err(EudkitError::Ir(
                            "trigger cannot have more than 16 conditions".to_string(),
                        ));
                    }
                    if lowered_actions.len() > 64 {
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
                        lowered_actions,
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
                            lower_single_act(&StockAction::SetVarConst {
                                var: *shift,
                                modifier: Modifier::Subtract,
                                value: 1,
                                mask: None,
                            }, map_ctx),
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
                ExpandedOp::MemReadSetup {
                    dst,
                    epd,
                    mask,
                    shift,
                    initial_val,
                    return_block,
                } => {
                    let ret_site = block_first_trigger[return_block];
                    let ret_site_addr = ValueExpr::ObjectAddress {
                        object: ret_site,
                        byte_offset: 0,
                    };
                    let dst_epd = var_storage[dst].value_epd();
                    let rt = read_table.as_ref().expect("read_table must be present");
                    let read_end_id = rt.read_end_id;

                    let acts_needed = if *initial_val != 0 { 3 } else { 2 };
                    match epd {
                        HirExpr::Const(c) => {
                            ensure_capacity(
                                acts_needed + 1,
                                &mut pending_actions,
                                &mut block_triggers,
                                &mut cur_obj_id,
                                &block.label,
                            );
                            pending_actions.push(make_set_deaths_action(
                                ValueExpr::FieldEpd(FieldRef::next_ptr(read_end_id.0)),
                                Modifier::SetTo,
                                ret_site_addr,
                                None,
                            ));
                            pending_actions.push(make_set_deaths_action(
                                ValueExpr::FieldEpd(FieldRef::new(
                                    read_end_id,
                                    TriggerSlotField::Action(0, ActField::Player1),
                                )),
                                Modifier::SetTo,
                                dst_epd,
                                None,
                            ));
                            if *initial_val != 0 {
                                pending_actions.push(make_set_deaths_action(
                                    ValueExpr::FieldEpd(FieldRef::new(
                                        read_end_id,
                                        TriggerSlotField::Action(0, ActField::Player2),
                                    )),
                                    Modifier::SetTo,
                                    ValueExpr::Const(*initial_val),
                                    None,
                                ));
                            }
                            pending_actions.push(make_set_deaths_action(
                                ValueExpr::Const(crate::hir::epd(CURRENT_PLAYER_ADDR)),
                                Modifier::SetTo,
                                ValueExpr::Const(*c),
                                None,
                            ));
                        }
                        HirExpr::Var(v) => {
                            let src_st = var_storage[v];
                            ensure_capacity(
                                acts_needed + 3,
                                &mut pending_actions,
                                &mut block_triggers,
                                &mut cur_obj_id,
                                &block.label,
                            );
                            pending_actions.push(make_set_deaths_action(
                                ValueExpr::FieldEpd(FieldRef::next_ptr(read_end_id.0)),
                                Modifier::SetTo,
                                ret_site_addr,
                                None,
                            ));
                            pending_actions.push(make_set_deaths_action(
                                ValueExpr::FieldEpd(FieldRef::new(
                                    read_end_id,
                                    TriggerSlotField::Action(0, ActField::Player1),
                                )),
                                Modifier::SetTo,
                                dst_epd,
                                None,
                            ));
                            if *initial_val != 0 {
                                pending_actions.push(make_set_deaths_action(
                                    ValueExpr::FieldEpd(FieldRef::new(
                                        read_end_id,
                                        TriggerSlotField::Action(0, ActField::Player2),
                                    )),
                                    Modifier::SetTo,
                                    ValueExpr::Const(*initial_val),
                                    None,
                                ));
                            }
                            let entry_obj = rt.entry(*mask, *shift);
                            let entry_addr = ValueExpr::ObjectAddress {
                                object: entry_obj,
                                byte_offset: 0,
                            };
                            pending_actions.push(make_vproc_set_dest(
                                src_st,
                                ValueExpr::Const(crate::hir::epd(CURRENT_PLAYER_ADDR)),
                                Modifier::SetTo,
                            ));
                            pending_actions.push(make_vproc_set_modifier(src_st, Modifier::SetTo));
                            pending_actions.push(make_vproc_set_nextptr(src_st, entry_addr));

                            let acts = std::mem::take(&mut pending_actions);
                            emit_custom_trigger(
                                format!("{}_memread_vproc_{}", block.label, cur_obj_id),
                                vproc_trigger_addr(src_st),
                                Vec::new(),
                                acts,
                                true,
                                &mut block_triggers,
                                &mut cur_obj_id,
                            );
                        }
                        _ => unreachable!("lowered epd must be Const or Var"),
                    }
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
            ExpandedTerminator::ReadTableJump { mask, shift } => {
                flush_pending_normal(
                    &mut pending_actions,
                    &mut block_triggers,
                    &mut cur_obj_id,
                    &block.label,
                );
                let rt = read_table.as_ref().expect("read_table must be present");
                let entry_obj = rt.entry(*mask, *shift);
                let entry_addr = ValueExpr::ObjectAddress {
                    object: entry_obj,
                    byte_offset: 0,
                };
                if block_triggers.is_empty() {
                    let id = ObjectId(cur_obj_id);
                    cur_obj_id += 1;
                    block_triggers.push(LirTrigger {
                        id,
                        label: Some(format!("{}_readtable_jump", block.label)),
                        prev_ptr: ValueExpr::Const(0),
                        next_ptr: entry_addr.clone(),
                        conditions: Vec::new(),
                        actions: Vec::new(),
                        flags: 4,
                    });
                }
                let last_trig = block_triggers.last_mut().expect("block must have triggers");
                if last_trig.next_ptr == ValueExpr::Const(0)
                    || last_trig.next_ptr == ValueExpr::Const(END_OF_TRIGGERS_SENTINEL as i32)
                {
                    last_trig.next_ptr = entry_addr;
                }
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

    if let Some(rt) = read_table {
        for t in rt.triggers {
            objects.push(LirObject::Trigger(t));
        }
    }

    debug_assert_eq!(
        cur_obj_id + fn_void_ret_trig.len() as u32 + read_table_count,
        trigger_count
    );

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
