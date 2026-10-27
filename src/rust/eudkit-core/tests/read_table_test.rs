use eudkit_core::hir::{
    epd, HirBasicBlock, HirExpr, HirFunction, HirModule, HirStmt, HirTerminator,
};
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
fn deduplicated_read_table_shares_bit_triggers_across_all_read_operations() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("read_table_dedup_test");
    let v_dw = hir.add_var("v_dw", 0);
    let v_epd = hir.add_var("v_epd", 0);
    let v_w = hir.add_var("v_w", 0);
    let v_b0 = hir.add_var("v_b0", 0);
    let v_b1 = hir.add_var("v_b1", 0);
    let v_b2 = hir.add_var("v_b2", 0);
    let v_b3 = hir.add_var("v_b3", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));

    // 1. Dword read at constant EPD
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_dw,
        epd: HirExpr::Const(epd(0x59CCA8)), // CUnit 0
        mask: 0xFFFF_FFFF,
        shift: 0,
        initial_val: 0,
    });

    // 2. EPD read (pointer to EPD conversion)
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_epd,
        epd: HirExpr::Const(epd(0x59CCA8 + 0x00C)), // unit.sprite pointer
        mask: 0xFFFF_FFFC,
        shift: -2,
        initial_val: epd(0),
    });

    // 3. Word read at subp 0
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_w,
        epd: HirExpr::Const(epd(0x59CCA8 + 0x028)), // unit.posX
        mask: 0x0000_FFFF,
        shift: 0,
        initial_val: 0,
    });

    // 4. Byte reads at subp 0, 1, 2, 3
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_b0,
        epd: HirExpr::Const(epd(0x59CCA8 + 0x020)),
        mask: 0x0000_00FF,
        shift: 0,
        initial_val: 0,
    });
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_b1,
        epd: HirExpr::Const(epd(0x59CCA8 + 0x020)),
        mask: 0x0000_FF00,
        shift: -8,
        initial_val: 0,
    });
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_b2,
        epd: HirExpr::Const(epd(0x59CCA8 + 0x020)),
        mask: 0x00FF_0000,
        shift: -16,
        initial_val: 0,
    });
    main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
        dst: v_b3,
        epd: HirExpr::Const(epd(0x59CCA8 + 0x020)),
        mask: 0xFF00_0000,
        shift: -24,
        initial_val: 0,
    });

    // 5. Another 3 dword reads to verify deduplication
    for i in 1..=3 {
        main_fn.block_mut(entry).push_stmt(HirStmt::MemRead {
            dst: v_dw,
            epd: HirExpr::Const(epd(0x59CCA8 + i * 336)),
            mask: 0xFFFF_FFFF,
            shift: 0,
            initial_val: 0,
        });
    }

    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);
    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let lir = session
        .lower_to_lir()
        .expect("should lower HIR with MemRead to LIR");

    // Total 10 read operations:
    // Without ReadTable deduplication, 10 reads * 32 = 320 triggers.
    // With deduplicated ReadTable in LIR, all reads share the same bit-read trigger table!
    let total_triggers = lir.trigger_count();
    assert!(
        total_triggers < 200,
        "Total triggers ({total_triggers}) must be far below 320 due to ReadTable deduplication"
    );

    // Verify LIR contains the ReadTable objects/triggers
    let lir_dump = session.dump_lir().expect("dump lir");
    assert!(
        lir_dump.contains("ReadTable") || lir_dump.contains("readtable"),
        "LIR dump must identify deduplicated ReadTable triggers"
    );
}
