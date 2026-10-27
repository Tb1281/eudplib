use crate::error::{EudkitError, Result};
use crate::hir::{
    BlockId, Comparison, HirBinOp, HirExpr, HirModule, HirStmt, HirTerminator, HirValue, Modifier,
    PlayerId, StockAction, StockCondition, VarId,
};
use crate::lir::cp_scheduler::schedule_cp_cursor;
use crate::lir::expr_lowerer::{eval_expr_to_var_or_const, lower_expr_into};
use crate::lir::types::{
    alloc_block, ActField, CondField, ExpandedBlock, ExpandedCfg, ExpandedOp,
    ExpandedTerminator, ScratchAllocator,
};
use std::collections::{HashMap, HashSet};

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
        | StockAction::FormatPrint { .. }
        | StockAction::PrintAt { .. }
        | StockAction::MinimapPing { .. }
        | StockAction::PlayWAV { .. }
        | StockAction::Comment { .. }
        | StockAction::PreserveTrigger
        | StockAction::Victory
        | StockAction::Defeat => true,
        StockAction::CreateUnit { .. }
        | StockAction::KillUnitAt { .. }
        | StockAction::KillUnit { .. }
        | StockAction::RemoveUnitAt { .. }
        | StockAction::MoveUnit { .. }
        | StockAction::GiveUnits { .. }
        | StockAction::Order { .. } => !matches!(
            cond,
            StockCondition::Command { .. } | StockCondition::Bring { .. }
        ),
        StockAction::SetResources { .. } => !matches!(cond, StockCondition::Accumulate { .. }),
        StockAction::SetScore { .. } => !matches!(cond, StockCondition::Score { .. }),
        StockAction::SetSwitch { switch: s1, .. } => match cond {
            StockCondition::Switch { switch: s2, .. } => s1 != s2,
            _ => true,
        },
        StockAction::MoveLocation { location: l1, .. } => match cond {
            StockCondition::Bring { location: l2, .. } => l1 != l2,
            _ => true,
        },
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

pub fn lower_and_expand_cfg(
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
                HirStmt::MemRead { dst, epd, mask, shift, initial_val } => {
                    let epd_lowered = if let HirExpr::Const(_) = epd {
                        epd.clone()
                    } else if let HirExpr::Var(_) = epd {
                        epd.clone()
                    } else {
                        let epd_scratch = scratch.acquire(hir);
                        lower_expr_into(epd, epd_scratch, hir, scratch, &mut out_blocks[cur_block.0 as usize].ops);
                        HirExpr::Var(epd_scratch)
                    };
                    let return_block = alloc_block(out_blocks, format!("{}_memread_ret", out_blocks[cur_block.0 as usize].label));
                    out_blocks[cur_block.0 as usize].ops.push(ExpandedOp::MemReadSetup {
                        dst: *dst,
                        epd: epd_lowered,
                        mask: *mask,
                        shift: *shift,
                        initial_val: *initial_val,
                        return_block,
                    });
                    out_blocks[cur_block.0 as usize].terminator = ExpandedTerminator::ReadTableJump {
                        mask: *mask,
                        shift: *shift,
                    };
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
                HirStmt::Reactive(_) => {
                    return Err(EudkitError::Ir(
                        "unexpected HirStmt::Reactive in CFG expansion; reactive rules must be lowered first".into(),
                    ));
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
                | ExpandedOp::CallIndirectSetup { return_block, .. }
                | ExpandedOp::MemReadSetup { return_block, .. } => {
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
            ExpandedTerminator::Return
            | ExpandedTerminator::FunctionReturn(_)
            | ExpandedTerminator::ReadTableJump { .. } => {}
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
                | ExpandedOp::CallIndirectSetup { return_block, .. }
                | ExpandedOp::MemReadSetup { return_block, .. } => {
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
            ExpandedTerminator::Return
            | ExpandedTerminator::FunctionReturn(_)
            | ExpandedTerminator::ReadTableJump { .. } => {}
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
                | ExpandedOp::CallIndirectSetup { return_block, .. }
                | ExpandedOp::MemReadSetup { return_block, .. } => {
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
            ExpandedTerminator::Return
            | ExpandedTerminator::FunctionReturn(_)
            | ExpandedTerminator::ReadTableJump { .. } => {}
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

    schedule_cp_cursor(&mut compact_blocks);

    Ok(ExpandedCfg {
        blocks: compact_blocks,
        fn_entry_block: new_fn_entry_block,
        fp_trampoline_block: new_fp_trampoline_block,
    })
}

