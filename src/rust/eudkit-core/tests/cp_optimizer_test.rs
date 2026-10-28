use eudkit_core::hir::{
    epd, HirBasicBlock, HirFunction, HirModule, HirStmt, HirTerminator, Modifier, PlayerId,
    StockAction, CURRENT_PLAYER_ADDR,
};
use eudkit_core::lir::LirObject;
use eudkit_core::{CompilationConfig, CompilationSession};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn cp_optimizer_groups_sequential_accesses_by_unit_stride_and_restores_cpcache() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("cp_optimizer_test");

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));

    // CUnit 0 base address is 0x59CCA8
    let cunit_epd = epd(0x59CCA8);

    // Three fields with the same residue r = 2:
    // Field A: offset 0x008 (EPD + 2) -> 12 * 0 + 2 (unit 0, r = 2)
    // Field B: offset 0x038 (EPD + 14) -> 12 * 1 + 2 (unit 1, r = 2)
    // Field C: offset 0x068 (EPD + 26) -> 12 * 2 + 2 (unit 2, r = 2)
    main_fn
        .block_mut(entry)
        .push_stmt(HirStmt::Action(StockAction::SetDeaths {
            player: PlayerId::Epd(cunit_epd + 2),
            modifier: Modifier::SetTo,
            value: eudkit_core::hir::HirValue::Const(100),
            unit: 0,
        }));
    main_fn
        .block_mut(entry)
        .push_stmt(HirStmt::Action(StockAction::SetDeaths {
            player: PlayerId::Epd(cunit_epd + 14),
            modifier: Modifier::SetTo,
            value: eudkit_core::hir::HirValue::Const(200),
            unit: 0,
        }));
    main_fn
        .block_mut(entry)
        .push_stmt(HirStmt::Action(StockAction::SetDeaths {
            player: PlayerId::Epd(cunit_epd + 26),
            modifier: Modifier::SetTo,
            value: eudkit_core::hir::HirValue::Const(300),
            unit: 0,
        }));

    // Field D with residue r = 5:
    // Field D: offset 0x014 (EPD + 5) -> 12 * 0 + 5 (unit 0, r = 5)
    main_fn
        .block_mut(entry)
        .push_stmt(HirStmt::Action(StockAction::SetDeaths {
            player: PlayerId::Epd(cunit_epd + 5),
            modifier: Modifier::SetTo,
            value: eudkit_core::hir::HirValue::Const(500),
            unit: 0,
        }));

    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);
    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let lir = session
        .lower_to_lir()
        .expect("should lower HIR with CP optimization to LIR");

    // Verify CP scheduling:
    // Look at triggers in the lowered LIR:
    // 1. Should use PlayerId::CurrentPlayer (13) for the struct accesses.
    // 2. Units should be 0, 1, 2 for the three r=2 accesses.
    // 3. Move from r=2 to r=5 should use relative adjustment (Add 3).
    // 4. cpcache must be restored (0x6509B0 restored to 0 / cached value).
    let mut cp_actions_found = 0;
    let mut restore_found = false;
    let mut unit_indices = Vec::new();

    for obj in lir.objects() {
        if let LirObject::Trigger(trig) = obj {
            for act in &trig.actions {
                // Check if act uses Player 13 (CurrentPlayer)
                if let eudkit_core::lir::ValueExpr::Const(13) = act.player1 {
                    cp_actions_found += 1;
                    unit_indices.push(act.unitid);
                }
                // Check if act restores 0x6509B0
                if let eudkit_core::lir::ValueExpr::Const(p) = act.player1 {
                    if p == epd(CURRENT_PLAYER_ADDR) && act.acttype == 45 {
                        if act.amount == Modifier::SetTo.encode() {
                            restore_found = true;
                        }
                    }
                }
            }
        }
    }

    assert!(
        cp_actions_found >= 3,
        "CP optimizer should schedule at least 3 accesses using Player 13 (CurrentPlayer), found {cp_actions_found}"
    );
    assert!(
        unit_indices.contains(&0) && unit_indices.contains(&1) && unit_indices.contains(&2),
        "Accesses should use 12 * u unit strides (units 0, 1, 2), got {unit_indices:?}"
    );
    assert!(
        restore_found,
        "CP optimizer must restore 0x6509B0 (cpcache) before exiting block"
    );
}
