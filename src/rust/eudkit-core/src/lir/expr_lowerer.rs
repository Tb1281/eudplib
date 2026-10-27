use crate::hir::{
    Comparison, HirBinOp, HirExpr, HirModule, HirUnaryOp, Modifier, StockAction, StockCondition,
    VarId,
};
use crate::lir::types::{ExpandedOp, ScratchAllocator};

pub(crate) fn push_var_const(out: &mut Vec<ExpandedOp>, var: VarId, modifier: Modifier, value: i32, mask: Option<u32>) {
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

pub fn lower_expr_into(
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

pub(crate) fn eval_expr_to_var_or_const(
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_const_expression() {
        let mut hir = HirModule::new("test");
        let mut scratch = ScratchAllocator::new();
        let mut out = Vec::new();
        let dst = hir.add_var("v0", 0);

        let expr = HirExpr::BinOp {
            op: HirBinOp::Add,
            lhs: Box::new(HirExpr::Const(10)),
            rhs: Box::new(HirExpr::Const(20)),
        };
        lower_expr_into(&expr, dst, &mut hir, &mut scratch, &mut out);
        assert_eq!(out.len(), 1);
        match &out[0] {
            ExpandedOp::LeafAction(StockAction::SetVarConst { var, value, .. }) => {
                assert_eq!(*var, dst);
                assert_eq!(*value, 30);
            }
            other => panic!("expected SetVarConst, got {other:?}"),
        }
    }

    #[test]
    fn test_scratch_allocator_reuse() {
        let mut hir = HirModule::new("test");
        let mut scratch = ScratchAllocator::new();

        let s1 = scratch.acquire(&mut hir);
        let s2 = scratch.acquire(&mut hir);
        assert_ne!(s1, s2);
        scratch.release(s1);
        let s3 = scratch.acquire(&mut hir);
        assert_eq!(s1, s3);
    }
}
