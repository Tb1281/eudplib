use crate::error::Result;
use crate::hir::{
    BlockId, FuncId, HirBasicBlock, HirExpr, HirFunction, HirModule, HirStmt, HirTerminator,
    InlineHint, StockCondition, VarId,
};
use std::collections::{HashMap, HashSet};

pub fn inline_functions(module: &mut HirModule) -> Result<()> {
    // Run iterative inlining up to 10 passes to handle nested calls
    for _pass in 0..10 {
        let inlinable_funcs = analyze_inlinable_functions(module);
        if inlinable_funcs.is_empty() {
            break;
        }

        let mut changed = false;
        let func_count = module.functions.len();
        for caller_idx in 0..func_count {
            if has_inlinable_calls(&module.functions[caller_idx], &inlinable_funcs) {
                inline_caller(caller_idx, module, &inlinable_funcs)?;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    Ok(())
}

fn analyze_inlinable_functions(module: &HirModule) -> HashSet<FuncId> {
    let mut call_counts: HashMap<FuncId, usize> = HashMap::new();
    let mut call_graph: HashMap<FuncId, HashSet<FuncId>> = HashMap::new();
    let mut func_ptr_targets: HashSet<FuncId> = HashSet::new();

    for fp in &module.func_ptrs {
        if let Some(target) = fp.initial_target {
            func_ptr_targets.insert(target);
        }
    }

    for func in &module.functions {
        let mut callees = HashSet::new();
        for block in &func.blocks {
            collect_calls_and_targets(&block.stmts, &mut callees, &mut func_ptr_targets);
        }
        for callee in &callees {
            *call_counts.entry(*callee).or_insert(0) += 1;
        }
        call_graph.insert(func.id, callees);
    }

    // Detect recursive functions
    let mut recursive = HashSet::new();
    for func in &module.functions {
        let mut visited = HashSet::new();
        if is_reachable(func.id, func.id, &call_graph, &mut visited, true) {
            recursive.insert(func.id);
        }
    }

    let mut inlinable = HashSet::new();
    for func in &module.functions {
        if func.is_main_loop {
            continue;
        }
        if recursive.contains(&func.id) {
            continue;
        }
        if func.inline_hint == InlineHint::Never {
            continue;
        }
        if func.inline_hint == InlineHint::Always {
            inlinable.insert(func.id);
            continue;
        }

        // Auto inline policy:
        let calls = call_counts.get(&func.id).copied().unwrap_or(0);
        let is_func_ptr_target = func_ptr_targets.contains(&func.id);
        let cost = calculate_function_cost(func);

        if calls == 1 && !is_func_ptr_target {
            inlinable.insert(func.id);
        } else if cost <= 12 {
            inlinable.insert(func.id);
        }
    }

    inlinable
}

fn collect_calls_and_targets(
    stmts: &[HirStmt],
    callees: &mut HashSet<FuncId>,
    func_ptr_targets: &mut HashSet<FuncId>,
) {
    for stmt in stmts {
        match stmt {
            HirStmt::Call { callee, .. } => {
                callees.insert(*callee);
            }
            HirStmt::FuncPtrAssign { target, .. } => {
                func_ptr_targets.insert(*target);
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_calls_and_targets(then_body, callees, func_ptr_targets);
                collect_calls_and_targets(else_body, callees, func_ptr_targets);
            }
            HirStmt::While { body, .. } => {
                collect_calls_and_targets(body, callees, func_ptr_targets);
            }
            _ => {}
        }
    }
}

fn is_reachable(
    current: FuncId,
    target: FuncId,
    graph: &HashMap<FuncId, HashSet<FuncId>>,
    visited: &mut HashSet<FuncId>,
    is_root: bool,
) -> bool {
    if !is_root && current == target {
        return true;
    }
    if !visited.insert(current) {
        return false;
    }
    if let Some(neighbors) = graph.get(&current) {
        for &next in neighbors {
            if is_reachable(next, target, graph, visited, false) {
                return true;
            }
        }
    }
    false
}

fn calculate_function_cost(func: &HirFunction) -> usize {
    let mut cost = 0;
    for block in &func.blocks {
        cost += calculate_stmts_cost(&block.stmts);
        if !matches!(block.terminator, HirTerminator::Return) {
            cost += 1;
        }
    }
    cost
}

fn calculate_stmts_cost(stmts: &[HirStmt]) -> usize {
    let mut cost = 0;
    for s in stmts {
        match s {
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                cost += 1 + calculate_stmts_cost(then_body) + calculate_stmts_cost(else_body);
            }
            HirStmt::While { body, .. } => {
                cost += 1 + calculate_stmts_cost(body);
            }
            _ => cost += 1,
        }
    }
    cost
}

fn has_inlinable_calls(func: &HirFunction, inlinable: &HashSet<FuncId>) -> bool {
    for block in &func.blocks {
        if stmts_contain_inlinable(&block.stmts, inlinable) {
            return true;
        }
    }
    false
}

fn stmts_contain_inlinable(stmts: &[HirStmt], inlinable: &HashSet<FuncId>) -> bool {
    for stmt in stmts {
        match stmt {
            HirStmt::Call { callee, .. } if inlinable.contains(callee) => return true,
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                if stmts_contain_inlinable(then_body, inlinable)
                    || stmts_contain_inlinable(else_body, inlinable)
                {
                    return true;
                }
            }
            HirStmt::While { body, .. } => {
                if stmts_contain_inlinable(body, inlinable) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn is_var_mutated_in_stmts(var: VarId, stmts: &[HirStmt]) -> bool {
    for stmt in stmts {
        match stmt {
            HirStmt::VarAssignConst { var: v, .. }
            | HirStmt::VarAddConst { var: v, .. }
            | HirStmt::VarSubConst { var: v, .. }
            | HirStmt::VarSetMaskConst { var: v, .. }
            | HirStmt::VarAssignExpr { var: v, .. }
                if *v == var =>
            {
                return true;
            }
            HirStmt::Call { rets, .. } | HirStmt::CallIndirect { rets, .. }
                if rets.contains(&var) =>
            {
                return true;
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                if is_var_mutated_in_stmts(var, then_body)
                    || is_var_mutated_in_stmts(var, else_body)
                {
                    return true;
                }
            }
            HirStmt::While { body, .. } => {
                if is_var_mutated_in_stmts(var, body) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn inline_caller(
    caller_idx: usize,
    module: &mut HirModule,
    inlinable: &HashSet<FuncId>,
) -> Result<()> {
    let caller = module.functions[caller_idx].clone();
    let mut new_blocks: Vec<HirBasicBlock> = Vec::new();

    for block in caller.blocks {
        let (expanded_stmts, _had_calls) =
            expand_stmts_inline(&block.stmts, module, inlinable)?;
        let mut new_block = HirBasicBlock::new(block.label.clone());
        new_block.id = BlockId(new_blocks.len() as u32);
        new_block.stmts = expanded_stmts;
        new_block.set_terminator(block.terminator.clone());
        new_blocks.push(new_block);
    }

    module.functions[caller_idx].blocks = new_blocks;
    Ok(())
}

fn expand_stmts_inline(
    stmts: &[HirStmt],
    module: &mut HirModule,
    inlinable: &HashSet<FuncId>,
) -> Result<(Vec<HirStmt>, bool)> {
    let mut out = Vec::new();
    let mut any_inlined = false;

    for stmt in stmts {
        match stmt {
            HirStmt::Call { callee, args, rets } if inlinable.contains(callee) => {
                let callee_fn = module.functions[callee.0 as usize].clone();
                let inlined_body = inline_call_into_stmts(&callee_fn, args, rets, module)?;
                out.extend(inlined_body);
                any_inlined = true;
            }
            HirStmt::If {
                conditions,
                then_body,
                else_body,
            } => {
                let (new_then, inlined_then) =
                    expand_stmts_inline(then_body, module, inlinable)?;
                let (new_else, inlined_else) =
                    expand_stmts_inline(else_body, module, inlinable)?;
                if inlined_then || inlined_else {
                    any_inlined = true;
                }
                out.push(HirStmt::If {
                    conditions: conditions.clone(),
                    then_body: new_then,
                    else_body: new_else,
                });
            }
            HirStmt::While { conditions, body } => {
                let (new_body, inlined_body) =
                    expand_stmts_inline(body, module, inlinable)?;
                if inlined_body {
                    any_inlined = true;
                }
                out.push(HirStmt::While {
                    conditions: conditions.clone(),
                    body: new_body,
                });
            }
            other => {
                out.push(other.clone());
            }
        }
    }

    Ok((out, any_inlined))
}

fn inline_call_into_stmts(
    callee: &HirFunction,
    args: &[HirExpr],
    rets: &[VarId],
    module: &mut HirModule,
) -> Result<Vec<HirStmt>> {
    if callee.blocks.is_empty() {
        return Ok(Vec::new());
    }

    // Collect all statements across blocks (for single-block and simple multi-block inline)
    let mut callee_stmts = Vec::new();
    for b in &callee.blocks {
        callee_stmts.extend(b.stmts.clone());
    }

    // Check parameter bindings & mutations
    let mut subst: HashMap<VarId, HirExpr> = HashMap::new();
    let mut prelude: Vec<HirStmt> = Vec::new();

    for (idx, &param) in callee.params.iter().enumerate() {
        let arg_expr = args
            .get(idx)
            .cloned()
            .unwrap_or(HirExpr::Const(0))
            .fold_constants();

        let mutated = is_var_mutated_in_stmts(param, &callee_stmts);
        match &arg_expr {
            HirExpr::Const(c) if !mutated => {
                subst.insert(param, HirExpr::Const(*c));
            }
            HirExpr::Var(v) if !mutated => {
                subst.insert(param, HirExpr::Var(*v));
            }
            _ => {
                // Parameter is mutated or a complex expression: allocate a local variable
                let fresh_param = module.add_var(format!("{}_p{}", callee.name, idx), 0);
                prelude.push(HirStmt::VarAssignExpr {
                    var: fresh_param,
                    expr: arg_expr,
                });
                subst.insert(param, HirExpr::Var(fresh_param));
            }
        }
    }

    // Rewrite statements with substituted variables and rewrite ReturnValues into assignments to caller's rets
    let mut inlined_stmts = prelude;

    // Check if callee has early returns or if ReturnValues is only at the end
    let return_count = callee.return_count;
    let mut return_exit_flag: Option<VarId> = None;

    // Count ReturnValues statements
    let mut num_returns = 0;
    count_returns(&callee_stmts, &mut num_returns);

    if num_returns > 1 {
        // Multi-return function: allocate an exit flag to bypass remaining statements after an early return
        let flag = module.add_var(format!("{}_ret_done", callee.name), 0);
        inlined_stmts.push(HirStmt::VarAssignConst {
            var: flag,
            value: 0,
        });
        return_exit_flag = Some(flag);
    }

    lower_inlined_stmts(
        &callee_stmts,
        rets,
        return_count,
        &subst,
        return_exit_flag,
        &mut inlined_stmts,
    )?;

    Ok(inlined_stmts)
}

fn count_returns(stmts: &[HirStmt], count: &mut usize) {
    for s in stmts {
        match s {
            HirStmt::ReturnValues(_) => *count += 1,
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                count_returns(then_body, count);
                count_returns(else_body, count);
            }
            HirStmt::While { body, .. } => {
                count_returns(body, count);
            }
            _ => {}
        }
    }
}

fn lower_inlined_stmts(
    stmts: &[HirStmt],
    caller_rets: &[VarId],
    expected_ret_count: usize,
    subst: &HashMap<VarId, HirExpr>,
    exit_flag: Option<VarId>,
    out: &mut Vec<HirStmt>,
) -> Result<()> {
    let mut current_subst = subst.clone();

    for stmt in stmts {
        match stmt {
            HirStmt::ReturnValues(values) => {
                for (k, val_expr) in values.iter().enumerate() {
                    let folded = val_expr.substitute_vars(&current_subst).fold_constants();
                    if let Some(&dest) = caller_rets.get(k) {
                        out.push(HirStmt::VarAssignExpr {
                            var: dest,
                            expr: folded,
                        });
                    }
                }
                if let Some(flag) = exit_flag {
                    out.push(HirStmt::VarAssignConst {
                        var: flag,
                        value: 1,
                    });
                }
            }
            HirStmt::VarAssignConst { var, value } => {
                let target = current_subst
                    .get(var)
                    .and_then(|e| match e {
                        HirExpr::Var(v) => Some(*v),
                        _ => None,
                    })
                    .unwrap_or(*var);
                current_subst.insert(*var, HirExpr::Const(*value));
                current_subst.insert(target, HirExpr::Const(*value));
                out.push(HirStmt::VarAssignConst {
                    var: target,
                    value: *value,
                });
            }
            HirStmt::VarAddConst { var, value } => {
                let target = current_subst
                    .get(var)
                    .and_then(|e| match e {
                        HirExpr::Var(v) => Some(*v),
                        _ => None,
                    })
                    .unwrap_or(*var);
                // Invalidate local propagation on mutation
                current_subst.remove(var);
                current_subst.remove(&target);
                out.push(HirStmt::VarAddConst {
                    var: target,
                    value: *value,
                });
            }
            HirStmt::VarSubConst { var, value } => {
                let target = current_subst
                    .get(var)
                    .and_then(|e| match e {
                        HirExpr::Var(v) => Some(*v),
                        _ => None,
                    })
                    .unwrap_or(*var);
                // Invalidate local propagation on mutation
                current_subst.remove(var);
                current_subst.remove(&target);
                out.push(HirStmt::VarSubConst {
                    var: target,
                    value: *value,
                });
            }
            HirStmt::VarSetMaskConst {
                var,
                modifier,
                value,
                mask,
            } => {
                let target = current_subst
                    .get(var)
                    .and_then(|e| match e {
                        HirExpr::Var(v) => Some(*v),
                        _ => None,
                    })
                    .unwrap_or(*var);
                current_subst.remove(var);
                current_subst.remove(&target);
                out.push(HirStmt::VarSetMaskConst {
                    var: target,
                    modifier: *modifier,
                    value: *value,
                    mask: *mask,
                });
            }
            HirStmt::VarAssignExpr { var, expr } => {
                let target = current_subst
                    .get(var)
                    .and_then(|e| match e {
                        HirExpr::Var(v) => Some(*v),
                        _ => None,
                    })
                    .unwrap_or(*var);
                let folded = expr.substitute_vars(&current_subst).fold_constants();
                current_subst.insert(*var, folded.clone());
                current_subst.insert(target, folded.clone());
                out.push(HirStmt::VarAssignExpr {
                    var: target,
                    expr: folded,
                });
            }
            HirStmt::If {
                conditions,
                then_body,
                else_body,
            } => {
                let mut new_then = Vec::new();
                let mut new_else = Vec::new();
                lower_inlined_stmts(
                    then_body,
                    caller_rets,
                    expected_ret_count,
                    &current_subst,
                    exit_flag,
                    &mut new_then,
                )?;
                lower_inlined_stmts(
                    else_body,
                    caller_rets,
                    expected_ret_count,
                    &current_subst,
                    exit_flag,
                    &mut new_else,
                )?;

                if let Some(flag) = exit_flag {
                    // Wrap if inside condition checking exit_flag == 0
                    out.push(HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: flag,
                            comparison: crate::hir::Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }],
                        then_body: vec![HirStmt::If {
                            conditions: conditions.clone(),
                            then_body: new_then,
                            else_body: new_else,
                        }],
                        else_body: Vec::new(),
                    });
                } else {
                    out.push(HirStmt::If {
                        conditions: conditions.clone(),
                        then_body: new_then,
                        else_body: new_else,
                    });
                }
            }
            HirStmt::While { conditions, body } => {
                let mut new_body = Vec::new();
                lower_inlined_stmts(
                    body,
                    caller_rets,
                    expected_ret_count,
                    &current_subst,
                    exit_flag,
                    &mut new_body,
                )?;
                if let Some(flag) = exit_flag {
                    out.push(HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: flag,
                            comparison: crate::hir::Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }],
                        then_body: vec![HirStmt::While {
                            conditions: conditions.clone(),
                            body: new_body,
                        }],
                        else_body: Vec::new(),
                    });
                } else {
                    out.push(HirStmt::While {
                        conditions: conditions.clone(),
                        body: new_body,
                    });
                }
            }
            other => {
                out.push(other.clone());
            }
        }
    }
    Ok(())
}
