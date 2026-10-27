use eudkit_core::hir::{HirBinOp, HirExpr, HirFunction, HirModule, HirStmt, InlineHint};
use eudkit_core::session::CompilationSession;
use std::path::PathBuf;

fn basemap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/outputmap/basemap/basemap_strx.scx")
}

#[test]
fn func_ptr_dynamic_dispatch_and_indirect_call_works() {
    let mut module = HirModule::new("test_func_ptr");

    // Two target functions with matching signature (1 arg, 1 return):
    // Target 1: fn_add5(x) -> x + 5
    let p_x = module.add_var("arg_x", 0);
    let mut f1 = HirFunction::new("fn_add5", false)
        .with_params(vec![p_x])
        .with_return_count(1)
        .with_inline_hint(InlineHint::Never);
    let mut bb1 = eudkit_core::hir::HirBasicBlock::new("entry");
    bb1.push_stmt(HirStmt::ReturnValues(vec![HirExpr::BinOp {
        op: HirBinOp::Add,
        lhs: Box::new(HirExpr::Var(p_x)),
        rhs: Box::new(HirExpr::Const(5)),
    }]));
    f1.add_block(bb1);
    let f1_id = module.add_function(f1);

    // Target 2: fn_mul3(x) -> x * 3
    let mut f2 = HirFunction::new("fn_mul3", false)
        .with_params(vec![p_x])
        .with_return_count(1)
        .with_inline_hint(InlineHint::Never);
    let mut bb2 = eudkit_core::hir::HirBasicBlock::new("entry");
    bb2.push_stmt(HirStmt::ReturnValues(vec![HirExpr::BinOp {
        op: HirBinOp::Mul,
        lhs: Box::new(HirExpr::Var(p_x)),
        rhs: Box::new(HirExpr::Const(3)),
    }]));
    f2.add_block(bb2);
    let f2_id = module.add_function(f2);

    // Register typed FuncPtr with initial target fn_add5
    let fp_id = module.add_func_ptr("op_ptr", (1, 1), Some(f1_id));

    // Caller in main_loop:
    // 1. Indirect call op_ptr(10) -> res1 (via initial target fn_add5)
    // 2. Dynamic assign: op_ptr.set(fn_mul3)
    // 3. Indirect call op_ptr(10) -> res2 (via dynamic target fn_mul3)
    let res1 = module.add_var("res1", 0);
    let res2 = module.add_var("res2", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let mut main_bb = eudkit_core::hir::HirBasicBlock::new("entry");

    main_bb.push_stmt(HirStmt::CallIndirect {
        ptr: fp_id,
        args: vec![HirExpr::Const(10)],
        rets: vec![res1],
    });

    main_bb.push_stmt(HirStmt::FuncPtrAssign {
        ptr: fp_id,
        target: f2_id,
    });

    main_bb.push_stmt(HirStmt::CallIndirect {
        ptr: fp_id,
        args: vec![HirExpr::Const(10)],
        rets: vec![res2],
    });

    main_fn.add_block(main_bb);
    module.add_function(main_fn);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_map = temp_dir.path().join("func_ptr_output.scx");

    let mut session = CompilationSession::from_map(&basemap_path(), eudkit_core::CompilationConfig::default()).expect("open session");
    session.set_hir_module(module);
    let lir = session.lower_to_lir().expect("lower to lir");

    assert!(lir.vproc_count() > 0, "indirect calls require return VProc slots");

    let report = session.compile_map(&output_map).expect("compile map");
    assert!(output_map.exists());
    assert!(report.payload_size > 0);
}
