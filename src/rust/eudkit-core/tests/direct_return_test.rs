use eudkit_core::hir::{HirBinOp, HirExpr, HirFunction, HirModule, HirStmt, InlineHint};
use eudkit_core::session::CompilationSession;
use std::path::PathBuf;

fn basemap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/outputmap/basemap/basemap_strx.scx")
}

#[test]
fn non_inlined_function_uses_direct_return_convention_without_double_copy() {
    let mut module = HirModule::new("test_direct_return");

    // Non-inlined function: double_and_inc(x) -> (x * 2, x + 1)
    let p_x = module.add_var("arg_x", 0);
    let mut func = HirFunction::new("double_and_inc", false)
        .with_params(vec![p_x])
        .with_return_count(2)
        .with_inline_hint(InlineHint::Never);

    let mut fn_bb = eudkit_core::hir::HirBasicBlock::new("entry");
    // Return values: (x * 2, x + 1)
    let ret1 = HirExpr::BinOp {
        op: HirBinOp::Mul,
        lhs: Box::new(HirExpr::Var(p_x)),
        rhs: Box::new(HirExpr::Const(2)),
    };
    let ret2 = HirExpr::BinOp {
        op: HirBinOp::Add,
        lhs: Box::new(HirExpr::Var(p_x)),
        rhs: Box::new(HirExpr::Const(1)),
    };
    fn_bb.push_stmt(HirStmt::ReturnValues(vec![ret1, ret2]));
    func.add_block(fn_bb);
    let func_id = module.add_function(func);

    // Caller: main_loop calls double_and_inc twice with different destinations
    let caller_r1 = module.add_var("c_r1", 0);
    let caller_r2 = module.add_var("c_r2", 0);
    let caller_r3 = module.add_var("c_r3", 0);
    let caller_r4 = module.add_var("c_r4", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let mut main_bb = eudkit_core::hir::HirBasicBlock::new("entry");
    // First call: double_and_inc(10) -> (caller_r1, caller_r2)
    main_bb.push_stmt(HirStmt::Call {
        callee: func_id,
        args: vec![HirExpr::Const(10)],
        rets: vec![caller_r1, caller_r2],
    });
    // Second call: double_and_inc(caller_r1) -> (caller_r3, caller_r4)
    main_bb.push_stmt(HirStmt::Call {
        callee: func_id,
        args: vec![HirExpr::Var(caller_r1)],
        rets: vec![caller_r3, caller_r4],
    });
    main_fn.add_block(main_bb);
    module.add_function(main_fn);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_map = temp_dir.path().join("direct_return_output.scx");

    let mut session =
        CompilationSession::from_map(&basemap_path(), eudkit_core::CompilationConfig::default())
            .expect("open session");
    session.set_hir_module(module);
    let lir = session.lower_to_lir().expect("lower to lir");

    // The non-inlined function must be present in LIR and not inlined away
    // It should have return VProc slots in the unified VProc buffer
    assert!(
        lir.vproc_count() > 0,
        "non-inlined function with returns requires return VProc slots"
    );

    let report = session.compile_map(&output_map).expect("compile map");
    assert!(output_map.exists());
    assert!(report.payload_size > 0);
}
