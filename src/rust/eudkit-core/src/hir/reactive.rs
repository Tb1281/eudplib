use crate::error::Result;
use crate::hir::{Comparison, HirModule, HirStmt, ReactiveRule, StockCondition};

/// Lowers all `HirStmt::Reactive` rules recursively across all functions and basic blocks
/// in the given `HirModule` into canonical 4-byte counter/guard variables and trigger conditions.
pub fn lower_reactive_rules(hir: &mut HirModule) -> Result<()> {
    for func_idx in 0..hir.functions.len() {
        for block_idx in 0..hir.functions[func_idx].blocks.len() {
            let stmts = std::mem::take(&mut hir.functions[func_idx].blocks[block_idx].stmts);
            let lowered = lower_reactive_stmts(stmts, hir);
            hir.functions[func_idx].blocks[block_idx].stmts = lowered;
        }
    }
    Ok(())
}

fn lower_reactive_stmts(stmts: Vec<HirStmt>, hir: &mut HirModule) -> Vec<HirStmt> {
    let mut out = Vec::with_capacity(stmts.len());
    for stmt in stmts {
        match stmt {
            HirStmt::Reactive(rule) => match rule {
                ReactiveRule::Once { cond, body } => {
                    let lowered_body = lower_reactive_stmts(body, hir);
                    let guard = hir.add_var("once_guard", 1);
                    let mut conds = vec![StockCondition::VarCmp {
                        var: guard,
                        comparison: Comparison::Exactly,
                        value: 1,
                        mask: None,
                    }];
                    conds.extend(cond);
                    let mut then_body = vec![HirStmt::VarAssignConst {
                        var: guard,
                        value: 0,
                    }];
                    then_body.extend(lowered_body);
                    out.push(HirStmt::If {
                        conditions: conds,
                        then_body,
                        else_body: Vec::new(),
                    });
                }
                ReactiveRule::When { cond, body } => {
                    let lowered_body = lower_reactive_stmts(body, hir);
                    out.push(HirStmt::If {
                        conditions: cond,
                        then_body: lowered_body,
                        else_body: Vec::new(),
                    });
                }
                ReactiveRule::Until { cond, body } => {
                    let lowered_body = lower_reactive_stmts(body, hir);
                    let guard = hir.add_var("until_guard", 0);
                    let inner_if = HirStmt::If {
                        conditions: cond,
                        then_body: vec![HirStmt::VarAssignConst {
                            var: guard,
                            value: 1,
                        }],
                        else_body: lowered_body,
                    };
                    out.push(HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: guard,
                            comparison: Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }],
                        then_body: vec![inner_if],
                        else_body: Vec::new(),
                    });
                }
                ReactiveRule::After { delay_ticks, body } => {
                    let lowered_body = lower_reactive_stmts(body, hir);
                    let timer = hir.add_var("after_timer", delay_ticks as i32);
                    let done = hir.add_var("after_done", 0);
                    let decr_stmt = HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: timer,
                            comparison: Comparison::AtLeast,
                            value: 1,
                            mask: None,
                        }],
                        then_body: vec![HirStmt::VarSubConst {
                            var: timer,
                            value: 1,
                        }],
                        else_body: Vec::new(),
                    };
                    let mut fire_body = vec![HirStmt::VarAssignConst {
                        var: done,
                        value: 1,
                    }];
                    fire_body.extend(lowered_body);
                    let fire_stmt = HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: timer,
                            comparison: Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }],
                        then_body: fire_body,
                        else_body: Vec::new(),
                    };
                    out.push(HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: done,
                            comparison: Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }],
                        then_body: vec![decr_stmt, fire_stmt],
                        else_body: Vec::new(),
                    });
                }
                ReactiveRule::Every { period_ticks, body } => {
                    let lowered_body = lower_reactive_stmts(body, hir);
                    let timer = hir.add_var("every_timer", period_ticks as i32);
                    let decr_stmt = HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: timer,
                            comparison: Comparison::AtLeast,
                            value: 1,
                            mask: None,
                        }],
                        then_body: vec![HirStmt::VarSubConst {
                            var: timer,
                            value: 1,
                        }],
                        else_body: Vec::new(),
                    };
                    let mut fire_body = vec![HirStmt::VarAssignConst {
                        var: timer,
                        value: period_ticks as i32,
                    }];
                    fire_body.extend(lowered_body);
                    let fire_stmt = HirStmt::If {
                        conditions: vec![StockCondition::VarCmp {
                            var: timer,
                            comparison: Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }],
                        then_body: fire_body,
                        else_body: Vec::new(),
                    };
                    out.push(decr_stmt);
                    out.push(fire_stmt);
                }
            },
            HirStmt::If {
                conditions,
                then_body,
                else_body,
            } => {
                out.push(HirStmt::If {
                    conditions,
                    then_body: lower_reactive_stmts(then_body, hir),
                    else_body: lower_reactive_stmts(else_body, hir),
                });
            }
            HirStmt::While { conditions, body } => {
                out.push(HirStmt::While {
                    conditions,
                    body: lower_reactive_stmts(body, hir),
                });
            }
            other => out.push(other),
        }
    }
    out
}
