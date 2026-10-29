//! Constant folding and algebraic simplification for HIR expressions.
//!
//! This module owns the optimization rules applied to `HirExpr` before the
//! Backend lowers HIR to LIR. `HirExpr::fold_constants` remains the public
//! entry point; it lives here so the HIR type module stays focused on the IR
//! contract.

use super::{HirBinOp, HirExpr, HirUnaryOp};

impl HirExpr {
    /// Folds constant subexpressions and applies algebraic simplifications
    /// that preserve the HIR expression's `u32` arithmetic semantics.
    pub fn fold_constants(&self) -> Self {
        match self {
            Self::Const(c) => Self::Const(*c),
            Self::Var(v) => Self::Var(*v),
            Self::UnaryOp { op, operand } => {
                let inner = operand.fold_constants();
                if let Self::Const(c) = inner {
                    let cu = c as u32;
                    let res = match op {
                        HirUnaryOp::Neg => cu.wrapping_neg(),
                        HirUnaryOp::BitNot => !cu,
                    };
                    return Self::Const(res as i32);
                }
                if let Self::UnaryOp {
                    op: inner_op,
                    operand: inner_operand,
                } = &inner
                {
                    if op == inner_op {
                        return (**inner_operand).clone();
                    }
                }
                Self::UnaryOp {
                    op: *op,
                    operand: Box::new(inner),
                }
            }
            Self::BinOp { op, lhs, rhs } => {
                let l = lhs.fold_constants();
                let r = rhs.fold_constants();
                if let (Self::Const(lc), Self::Const(rc)) = (&l, &r) {
                    let lu = *lc as u32;
                    let ru = *rc as u32;
                    let res = match op {
                        HirBinOp::Add => lu.wrapping_add(ru),
                        HirBinOp::Sub => lu.wrapping_sub(ru),
                        HirBinOp::SubSat => lu.saturating_sub(ru),
                        HirBinOp::Mul => lu.wrapping_mul(ru),
                        HirBinOp::Div => {
                            if ru == 0 {
                                u32::MAX
                            } else {
                                lu / ru
                            }
                        }
                        HirBinOp::Rem => {
                            if ru == 0 {
                                lu
                            } else {
                                lu % ru
                            }
                        }
                        HirBinOp::BitAnd => lu & ru,
                        HirBinOp::BitOr => lu | ru,
                        HirBinOp::BitXor => lu ^ ru,
                        HirBinOp::Shl => {
                            if ru >= 32 {
                                0
                            } else {
                                lu << ru
                            }
                        }
                        HirBinOp::Shr => {
                            if ru >= 32 {
                                0
                            } else {
                                lu >> ru
                            }
                        }
                    };
                    return Self::Const(res as i32);
                }

                // Algebraic simplifications with one constant operand
                match (op, &l, &r) {
                    (HirBinOp::Add, _, Self::Const(0)) => return l,
                    (HirBinOp::Add, Self::Const(0), _) => return r,
                    (HirBinOp::Sub, _, Self::Const(0)) => return l,
                    (HirBinOp::Sub, Self::Const(0), _) => {
                        return Self::UnaryOp {
                            op: HirUnaryOp::Neg,
                            operand: Box::new(r),
                        };
                    }
                    (HirBinOp::Sub, Self::Var(v1), Self::Var(v2)) if v1 == v2 => {
                        return Self::Const(0);
                    }
                    (HirBinOp::SubSat, _, Self::Const(0)) => return l,
                    (HirBinOp::SubSat, Self::Const(0), _) => return Self::Const(0),
                    (HirBinOp::SubSat, Self::Var(v1), Self::Var(v2)) if v1 == v2 => {
                        return Self::Const(0);
                    }
                    (HirBinOp::Mul, _, Self::Const(0)) | (HirBinOp::Mul, Self::Const(0), _) => {
                        return Self::Const(0);
                    }
                    (HirBinOp::Mul, _, Self::Const(1)) => return l,
                    (HirBinOp::Mul, Self::Const(1), _) => return r,
                    (HirBinOp::Mul, _, Self::Const(c)) if (*c as u32).is_power_of_two() => {
                        let k = (*c as u32).trailing_zeros();
                        return Self::BinOp {
                            op: HirBinOp::Shl,
                            lhs: Box::new(l),
                            rhs: Box::new(Self::Const(k as i32)),
                        };
                    }
                    (HirBinOp::Mul, Self::Const(c), _) if (*c as u32).is_power_of_two() => {
                        let k = (*c as u32).trailing_zeros();
                        return Self::BinOp {
                            op: HirBinOp::Shl,
                            lhs: Box::new(r),
                            rhs: Box::new(Self::Const(k as i32)),
                        };
                    }
                    (HirBinOp::Div, _, Self::Const(1)) => return l,
                    (HirBinOp::Div, _, Self::Const(c)) if (*c as u32).is_power_of_two() => {
                        let k = (*c as u32).trailing_zeros();
                        return Self::BinOp {
                            op: HirBinOp::Shr,
                            lhs: Box::new(l),
                            rhs: Box::new(Self::Const(k as i32)),
                        };
                    }
                    (HirBinOp::Rem, _, Self::Const(1)) => return Self::Const(0),
                    (HirBinOp::Rem, _, Self::Const(c)) if (*c as u32).is_power_of_two() => {
                        let mask = (*c as u32) - 1;
                        return Self::BinOp {
                            op: HirBinOp::BitAnd,
                            lhs: Box::new(l),
                            rhs: Box::new(Self::Const(mask as i32)),
                        };
                    }
                    (HirBinOp::BitAnd, _, Self::Const(0))
                    | (HirBinOp::BitAnd, Self::Const(0), _) => return Self::Const(0),
                    (HirBinOp::BitAnd, _, Self::Const(-1)) => return l,
                    (HirBinOp::BitAnd, Self::Const(-1), _) => return r,
                    (HirBinOp::BitOr, _, Self::Const(0)) => return l,
                    (HirBinOp::BitOr, Self::Const(0), _) => return r,
                    (HirBinOp::BitOr, _, Self::Const(-1))
                    | (HirBinOp::BitOr, Self::Const(-1), _) => return Self::Const(-1),
                    (HirBinOp::BitXor, _, Self::Const(0)) => return l,
                    (HirBinOp::BitXor, Self::Const(0), _) => return r,
                    (HirBinOp::BitXor, Self::Var(v1), Self::Var(v2)) if v1 == v2 => {
                        return Self::Const(0);
                    }
                    (HirBinOp::Shl | HirBinOp::Shr, _, Self::Const(0)) => return l,
                    (HirBinOp::Shl | HirBinOp::Shr, _, Self::Const(c)) if (*c as u32) >= 32 => {
                        return Self::Const(0);
                    }
                    _ => {}
                }

                Self::BinOp {
                    op: *op,
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HirExpr;
    use crate::hir::{HirBinOp, HirUnaryOp, VarId};

    fn var(id: u32) -> HirExpr {
        HirExpr::Var(VarId(id))
    }

    fn bin(op: HirBinOp, lhs: HirExpr, rhs: HirExpr) -> HirExpr {
        HirExpr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn unary(op: HirUnaryOp, operand: HirExpr) -> HirExpr {
        HirExpr::UnaryOp {
            op,
            operand: Box::new(operand),
        }
    }

    #[test]
    fn folds_constant_arithmetic_with_u32_wrapping_semantics() {
        assert_eq!(
            bin(HirBinOp::Add, HirExpr::Const(2), HirExpr::Const(3)).fold_constants(),
            HirExpr::Const(5)
        );
        assert_eq!(
            bin(HirBinOp::Sub, HirExpr::Const(2), HirExpr::Const(5)).fold_constants(),
            HirExpr::Const(-3)
        );
        assert_eq!(
            bin(HirBinOp::Mul, HirExpr::Const(7), HirExpr::Const(6)).fold_constants(),
            HirExpr::Const(42)
        );
        assert_eq!(
            bin(HirBinOp::Add, HirExpr::Const(i32::MAX), HirExpr::Const(1)).fold_constants(),
            HirExpr::Const(i32::MIN)
        );
        assert_eq!(
            bin(HirBinOp::SubSat, HirExpr::Const(3), HirExpr::Const(9)).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(
                HirBinOp::BitXor,
                HirExpr::Const(0b1100),
                HirExpr::Const(0b1010)
            )
            .fold_constants(),
            HirExpr::Const(0b0110)
        );
    }

    #[test]
    fn folds_division_remainder_and_shift_edge_cases() {
        assert_eq!(
            bin(HirBinOp::Div, HirExpr::Const(7), HirExpr::Const(0)).fold_constants(),
            HirExpr::Const(-1)
        );
        assert_eq!(
            bin(HirBinOp::Rem, HirExpr::Const(7), HirExpr::Const(0)).fold_constants(),
            HirExpr::Const(7)
        );
        assert_eq!(
            bin(HirBinOp::Div, HirExpr::Const(9), HirExpr::Const(2)).fold_constants(),
            HirExpr::Const(4)
        );
        assert_eq!(
            bin(HirBinOp::Shl, HirExpr::Const(1), HirExpr::Const(31)).fold_constants(),
            HirExpr::Const(i32::MIN)
        );
        assert_eq!(
            bin(HirBinOp::Shl, HirExpr::Const(1), HirExpr::Const(32)).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(HirBinOp::Shr, HirExpr::Const(-1), HirExpr::Const(40)).fold_constants(),
            HirExpr::Const(0)
        );
    }

    #[test]
    fn folds_unary_constants_and_collapses_double_negation() {
        assert_eq!(
            unary(HirUnaryOp::Neg, HirExpr::Const(5)).fold_constants(),
            HirExpr::Const(-5)
        );
        assert_eq!(
            unary(HirUnaryOp::BitNot, HirExpr::Const(0)).fold_constants(),
            HirExpr::Const(-1)
        );
        assert_eq!(
            unary(HirUnaryOp::Neg, unary(HirUnaryOp::Neg, var(1))).fold_constants(),
            var(1)
        );
        assert_eq!(
            unary(HirUnaryOp::BitNot, unary(HirUnaryOp::BitNot, var(1))).fold_constants(),
            var(1)
        );
        assert_eq!(
            unary(HirUnaryOp::Neg, unary(HirUnaryOp::BitNot, var(1))).fold_constants(),
            unary(HirUnaryOp::Neg, unary(HirUnaryOp::BitNot, var(1)))
        );
    }

    #[test]
    fn simplifies_identity_and_annihilator_rules() {
        let x = var(1);
        let y = var(2);

        assert_eq!(
            bin(HirBinOp::Add, x.clone(), HirExpr::Const(0)).fold_constants(),
            x
        );
        assert_eq!(
            bin(HirBinOp::Add, HirExpr::Const(0), y.clone()).fold_constants(),
            y
        );
        assert_eq!(
            bin(HirBinOp::Sub, x.clone(), HirExpr::Const(0)).fold_constants(),
            x
        );
        assert_eq!(
            bin(HirBinOp::Sub, HirExpr::Const(0), y.clone()).fold_constants(),
            unary(HirUnaryOp::Neg, y.clone())
        );
        assert_eq!(
            bin(HirBinOp::Sub, x.clone(), x.clone()).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(HirBinOp::SubSat, x.clone(), x.clone()).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(HirBinOp::Mul, x.clone(), HirExpr::Const(0)).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(HirBinOp::Mul, x.clone(), HirExpr::Const(1)).fold_constants(),
            x
        );
        assert_eq!(
            bin(HirBinOp::Div, x.clone(), HirExpr::Const(1)).fold_constants(),
            x
        );
        assert_eq!(
            bin(HirBinOp::Rem, x.clone(), HirExpr::Const(1)).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(HirBinOp::BitAnd, x.clone(), HirExpr::Const(-1)).fold_constants(),
            x
        );
        assert_eq!(
            bin(HirBinOp::BitOr, x.clone(), HirExpr::Const(-1)).fold_constants(),
            HirExpr::Const(-1)
        );
        assert_eq!(
            bin(HirBinOp::BitXor, x.clone(), x.clone()).fold_constants(),
            HirExpr::Const(0)
        );
        assert_eq!(
            bin(HirBinOp::Shl, x.clone(), HirExpr::Const(0)).fold_constants(),
            x
        );
        assert_eq!(
            bin(HirBinOp::Shl, x, HirExpr::Const(32)).fold_constants(),
            HirExpr::Const(0)
        );
    }

    #[test]
    fn rewrites_power_of_two_multiplication_and_division_to_shifts() {
        assert_eq!(
            bin(HirBinOp::Mul, var(1), HirExpr::Const(8)).fold_constants(),
            bin(HirBinOp::Shl, var(1), HirExpr::Const(3))
        );
        assert_eq!(
            bin(HirBinOp::Mul, HirExpr::Const(16), var(1)).fold_constants(),
            bin(HirBinOp::Shl, var(1), HirExpr::Const(4))
        );
        assert_eq!(
            bin(HirBinOp::Div, var(1), HirExpr::Const(4)).fold_constants(),
            bin(HirBinOp::Shr, var(1), HirExpr::Const(2))
        );
        assert_eq!(
            bin(HirBinOp::Rem, var(1), HirExpr::Const(8)).fold_constants(),
            bin(HirBinOp::BitAnd, var(1), HirExpr::Const(7))
        );
    }

    #[test]
    fn keeps_unfoldable_expressions_structural() {
        let expr = bin(HirBinOp::Add, var(1), var(2));
        assert_eq!(expr.fold_constants(), expr);

        let nested = bin(
            HirBinOp::Mul,
            bin(HirBinOp::Add, var(1), HirExpr::Const(1)),
            HirExpr::Const(3),
        );
        assert_eq!(
            nested.fold_constants(),
            bin(
                HirBinOp::Mul,
                bin(HirBinOp::Add, var(1), HirExpr::Const(1)),
                HirExpr::Const(3)
            )
        );
    }
}
