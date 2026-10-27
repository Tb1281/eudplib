use eudkit_core::hir::{
    CmpOp, Comparison, HirBinOp, HirExpr, StockCondition, VarId,
};

#[test]
fn test_comparison_zero_bound_checks() {
    let x = HirExpr::Var(VarId(0));

    // x < 0 is mathematically impossible in unsigned 32-bit math -> Never
    let cond_lt_0 = StockCondition::from_cmp(x.clone(), CmpOp::Lt, HirExpr::Const(0));
    assert_eq!(cond_lt_0, StockCondition::Never);

    // x >= 0 is always true in unsigned 32-bit math -> Always
    let cond_ge_0 = StockCondition::from_cmp(x.clone(), CmpOp::Ge, HirExpr::Const(0));
    assert_eq!(cond_ge_0, StockCondition::Always);

    // x > u32::MAX is mathematically impossible -> Never
    let cond_gt_max = StockCondition::from_cmp(x.clone(), CmpOp::Gt, HirExpr::Const(-1));
    assert_eq!(cond_gt_max, StockCondition::Never);

    // x <= u32::MAX is always true -> Always
    let cond_le_max = StockCondition::from_cmp(x.clone(), CmpOp::Le, HirExpr::Const(-1));
    assert_eq!(cond_le_max, StockCondition::Always);
}

#[test]
fn test_comparison_constant_folding() {
    // 5 < 10 -> Always
    assert_eq!(
        StockCondition::from_cmp(HirExpr::Const(5), CmpOp::Lt, HirExpr::Const(10)),
        StockCondition::Always
    );

    // 10 < 5 -> Never
    assert_eq!(
        StockCondition::from_cmp(HirExpr::Const(10), CmpOp::Lt, HirExpr::Const(5)),
        StockCondition::Never
    );

    // 42 == 42 -> Always
    assert_eq!(
        StockCondition::from_cmp(HirExpr::Const(42), CmpOp::Eq, HirExpr::Const(42)),
        StockCondition::Always
    );

    // 42 != 42 -> Never
    assert_eq!(
        StockCondition::from_cmp(HirExpr::Const(42), CmpOp::Ne, HirExpr::Const(42)),
        StockCondition::Never
    );
}

#[test]
fn test_comparison_var_and_const() {
    let v = VarId(1);
    let x = HirExpr::Var(v);

    // x < 10 -> x <= 9 (AtMost 9)
    let cond_lt = StockCondition::from_cmp(x.clone(), CmpOp::Lt, HirExpr::Const(10));
    assert_eq!(
        cond_lt,
        StockCondition::VarCmp {
            var: v,
            comparison: Comparison::AtMost,
            value: 9,
            mask: None,
        }
    );

    // x > 10 -> x >= 11 (AtLeast 11)
    let cond_gt = StockCondition::from_cmp(x.clone(), CmpOp::Gt, HirExpr::Const(10));
    assert_eq!(
        cond_gt,
        StockCondition::VarCmp {
            var: v,
            comparison: Comparison::AtLeast,
            value: 11,
            mask: None,
        }
    );

    // x == 10 -> Exactly 10
    let cond_eq = StockCondition::from_cmp(x.clone(), CmpOp::Eq, HirExpr::Const(10));
    assert_eq!(
        cond_eq,
        StockCondition::VarCmp {
            var: v,
            comparison: Comparison::Exactly,
            value: 10,
            mask: None,
        }
    );
}

#[test]
fn test_comparison_dynamic_expressions_use_saturating_subtraction() {
    let a = HirExpr::Var(VarId(1));
    let b = HirExpr::Var(VarId(2));

    // a < b -> (b -sat a) >= 1
    let cond_lt = StockCondition::from_cmp(a.clone(), CmpOp::Lt, b.clone());
    let expected_sat_diff = HirExpr::BinOp {
        op: HirBinOp::SubSat,
        lhs: Box::new(b.clone()),
        rhs: Box::new(a.clone()),
    };
    assert_eq!(
        cond_lt,
        StockCondition::ExprCmp {
            lhs: expected_sat_diff,
            comparison: Comparison::AtLeast,
            rhs: HirExpr::Const(1),
        }
    );

    // a > b -> (a -sat b) >= 1
    let cond_gt = StockCondition::from_cmp(a.clone(), CmpOp::Gt, b.clone());
    let expected_gt_diff = HirExpr::BinOp {
        op: HirBinOp::SubSat,
        lhs: Box::new(a.clone()),
        rhs: Box::new(b.clone()),
    };
    assert_eq!(
        cond_gt,
        StockCondition::ExprCmp {
            lhs: expected_gt_diff,
            comparison: Comparison::AtLeast,
            rhs: HirExpr::Const(1),
        }
    );

    // a != b -> (a - b) >= 1
    let cond_ne = StockCondition::from_cmp(a.clone(), CmpOp::Ne, b.clone());
    let expected_sub = HirExpr::BinOp {
        op: HirBinOp::Sub,
        lhs: Box::new(a.clone()),
        rhs: Box::new(b.clone()),
    };
    assert_eq!(
        cond_ne,
        StockCondition::ExprCmp {
            lhs: expected_sub,
            comparison: Comparison::AtLeast,
            rhs: HirExpr::Const(1),
        }
    );
}
