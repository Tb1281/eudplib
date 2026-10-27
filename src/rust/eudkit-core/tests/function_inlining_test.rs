use eudkit_core::hir::{
    Comparison, HirBinOp, HirExpr, HirFunction, HirModule, HirStmt, InlineHint, StockCondition,
};
use eudkit_core::session::CompilationSession;
use std::fs;
use std::path::PathBuf;

fn basemap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/outputmap/basemap/basemap_strx.scx")
}

#[test]
fn inlines_nested_functions_and_rewrites_multi_return_branches() {
    let mut module = HirModule::new("test_nested_inlining");

    // Leaf helper: double(x) = x * 2
    let p_x = module.add_var("double_x", 0);
    let mut double_fn = HirFunction::new("double", false)
        .with_params(vec![p_x])
        .with_return_count(1)
        .with_inline_hint(InlineHint::Always);
    let mut double_bb = eudkit_core::hir::HirBasicBlock::new("entry");
    double_bb.push_stmt(HirStmt::ReturnValues(vec![HirExpr::BinOp {
        op: HirBinOp::Mul,
        lhs: Box::new(HirExpr::Var(p_x)),
        rhs: Box::new(HirExpr::Const(2)),
    }]));
    double_fn.add_block(double_bb);
    let double_id = module.add_function(double_fn);

    // Caller helper: clamp_and_double(a):
    // if a > 50: return double(50)
    // return double(a)
    let p_a = module.add_var("clamp_a", 0);
    let mut clamp_fn = HirFunction::new("clamp_and_double", false)
        .with_params(vec![p_a])
        .with_return_count(1)
        .with_inline_hint(InlineHint::Auto);
    let mut clamp_bb = eudkit_core::hir::HirBasicBlock::new("entry");

    let early_ret_var = module.add_var("early_ret", 0);
    let norm_ret_var = module.add_var("norm_ret", 0);

    clamp_bb.push_stmt(HirStmt::If {
        conditions: vec![StockCondition::VarCmp {
            var: p_a,
            comparison: Comparison::AtLeast,
            value: 51,
            mask: None,
        }],
        then_body: vec![
            HirStmt::Call {
                callee: double_id,
                args: vec![HirExpr::Const(50)],
                rets: vec![early_ret_var],
            },
            HirStmt::ReturnValues(vec![HirExpr::Var(early_ret_var)]),
        ],
        else_body: Vec::new(),
    });
    clamp_bb.push_stmt(HirStmt::Call {
        callee: double_id,
        args: vec![HirExpr::Var(p_a)],
        rets: vec![norm_ret_var],
    });
    clamp_bb.push_stmt(HirStmt::ReturnValues(vec![HirExpr::Var(norm_ret_var)]));
    clamp_fn.add_block(clamp_bb);
    let clamp_id = module.add_function(clamp_fn);

    // Main function:
    // res1 = clamp_and_double(60)
    let out_var = module.add_var("res1", 0);
    let mut main_fn = HirFunction::new_main_loop("main");
    let mut main_bb = eudkit_core::hir::HirBasicBlock::new("entry");
    main_bb.push_stmt(HirStmt::Call {
        callee: clamp_id,
        args: vec![HirExpr::Const(60)],
        rets: vec![out_var],
    });
    main_fn.add_block(main_bb);
    module.add_function(main_fn);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_map = temp_dir.path().join("nested_inlined_output.scx");

    let mut session = CompilationSession::from_map(&basemap_path(), eudkit_core::CompilationConfig::default()).expect("open session");
    session.set_hir_module(module);
    let lir = session.lower_to_lir().expect("lower to lir");
    assert_eq!(lir.vproc_count(), 0, "all nested constant operations must fold with 0 VProcs");

    let _report = session.compile_map(&output_map).expect("compile map");
    assert!(output_map.exists());
}

#[test]
fn inlines_small_helper_function_with_zero_call_overhead_and_folds_constants() {
    let mut module = HirModule::new("test_inlining");

    // Helper: add_mul(x, y) = (x + y) * 2
    let param_x = module.add_var("x", 0);
    let param_y = module.add_var("y", 0);

    let mut helper_fn = HirFunction::new("add_mul", false)
        .with_params(vec![param_x, param_y])
        .with_return_count(1)
        .with_inline_hint(InlineHint::Auto);

    let mut helper_bb = eudkit_core::hir::HirBasicBlock::new("entry");
    // return (x + y) * 2
    let expr = HirExpr::BinOp {
        op: HirBinOp::Mul,
        lhs: Box::new(HirExpr::BinOp {
            op: HirBinOp::Add,
            lhs: Box::new(HirExpr::Var(param_x)),
            rhs: Box::new(HirExpr::Var(param_y)),
        }),
        rhs: Box::new(HirExpr::Const(2)),
    };
    helper_bb.push_stmt(HirStmt::ReturnValues(vec![expr]));
    helper_fn.add_block(helper_bb);
    let helper_id = module.add_function(helper_fn);

    // Main function:
    // res = add_mul(10, 5)  --> should fold to 30!
    let res_var = module.add_var("res", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let mut main_bb = eudkit_core::hir::HirBasicBlock::new("entry");
    main_bb.push_stmt(HirStmt::Call {
        callee: helper_id,
        args: vec![HirExpr::Const(10), HirExpr::Const(5)],
        rets: vec![res_var],
    });
    main_fn.add_block(main_bb);
    module.add_function(main_fn);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_map = temp_dir.path().join("inlined_output.scx");

    let mut session = CompilationSession::from_map(&basemap_path(), eudkit_core::CompilationConfig::default()).expect("open session");
    session.set_hir_module(module);
    let lir = session.lower_to_lir().expect("lower to lir");
    assert_eq!(lir.vproc_count(), 0, "inlined constant function must allocate 0 VProcs");

    let _report = session.compile_map(&output_map).expect("compile map");
    assert!(output_map.exists());
    let metadata = fs::metadata(&output_map).expect("metadata");
    assert!(metadata.len() > 0);
}
