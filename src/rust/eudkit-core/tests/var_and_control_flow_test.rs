use eudkit_core::hir::{
    Comparison, HirBasicBlock, HirFunction, HirModule, HirStmt, HirTerminator, Modifier, PlayerId,
    ResourceType, StockAction, StockCondition,
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
fn lowers_constant_only_variables_to_4b_bytebuf_cells_with_zero_vprocs_and_coalesces_actions() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("var_coalesce_test");
    let counter = hir.add_var("counter", 0);
    let phase = hir.add_var("phase", 42);
    let flags = hir.add_var("flags", 0x00FF);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));

    // 70 consecutive unconditional variable/action updates -> should coalesce into 64 + 6 = 2 triggers
    for i in 0..68 {
        main_fn
            .block_mut(entry)
            .push_stmt(HirStmt::VarAddConst { var: counter, value: i + 1 });
    }
    main_fn
        .block_mut(entry)
        .push_stmt(HirStmt::VarSubConst { var: phase, value: 2 });
    main_fn
        .block_mut(entry)
        .push_stmt(HirStmt::VarSetMaskConst {
            var: flags,
            modifier: Modifier::SetTo,
            value: 0x0010,
            mask: 0x00FF,
        });

    // Structured If without else: single conditional trigger
    main_fn.block_mut(entry).push_stmt(HirStmt::If {
        conditions: vec![StockCondition::VarCmp {
            var: counter,
            comparison: Comparison::AtLeast,
            value: 100,
            mask: None,
        }],
        then_body: vec![
            HirStmt::VarAssignConst {
                var: counter,
                value: 0,
            },
            HirStmt::Action(StockAction::SetResources {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                amount: 250,
                resource: ResourceType::Ore,
            }),
        ],
        else_body: Vec::new(),
    });

    // Structured While loop with If/Else, Continue, and Break
    main_fn.block_mut(entry).push_stmt(HirStmt::While {
        conditions: vec![StockCondition::VarCmp {
            var: phase,
            comparison: Comparison::AtLeast,
            value: 1,
            mask: None,
        }],
        body: vec![
            HirStmt::VarSubConst {
                var: phase,
                value: 1,
            },
            HirStmt::If {
                conditions: vec![StockCondition::VarCmp {
                    var: phase,
                    comparison: Comparison::Exactly,
                    value: 5,
                    mask: None,
                }],
                then_body: vec![
                    HirStmt::VarAddConst {
                        var: counter,
                        value: 10,
                    },
                    HirStmt::Continue,
                ],
                else_body: vec![HirStmt::VarAddConst {
                    var: counter,
                    value: 1,
                }],
            },
            HirStmt::If {
                conditions: vec![StockCondition::VarCmp {
                    var: counter,
                    comparison: Comparison::AtLeast,
                    value: 500,
                    mask: None,
                }],
                then_body: vec![HirStmt::Break],
                else_body: Vec::new(),
            },
        ],
    });

    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);

    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let hir_dump = session.dump_hir();
    assert!(hir_dump.contains("var @counter"));
    assert!(hir_dump.contains("var @phase"));
    assert!(hir_dump.contains("while"));

    let lir = session
        .lower_to_lir()
        .expect("should lower HIR with vars and structured CF to LIR")
        .clone();

    // 1. Verify 0 VProc triggers allocated
    assert_eq!(lir.vproc_count(), 0, "constant-only variables must allocate 0 VProc triggers");

    // 2. Verify all 3 variables lowered into 4-byte ByteBuf cells with initial values
    let byte_bufs: Vec<_> = lir
        .objects()
        .iter()
        .filter_map(|o| match o {
            LirObject::ByteBuf(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(byte_bufs.len(), 3, "3 variables should lower to 3 4-byte ByteBuf objects");
    assert_eq!(byte_bufs[0].data, 0i32.to_le_bytes());
    assert_eq!(byte_bufs[1].data, 42i32.to_le_bytes());
    assert_eq!(byte_bufs[2].data, 0x00FFi32.to_le_bytes());

    // 3. Verify the first trigger coalesced 64 actions
    let triggers: Vec<_> = lir
        .objects()
        .iter()
        .filter_map(|o| match o {
            LirObject::Trigger(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggers[0].actions.len(),
        64,
        "first 64 consecutive unconditional actions must coalesce into a single 64-action trigger"
    );
    assert_eq!(
        triggers[1].actions.len(),
        6,
        "remaining 6 unconditional actions must coalesce into the second trigger"
    );

    // 4. Verify end-to-end map compilation succeeds and is deterministic
    let temp_dir = tempfile::tempdir().unwrap();
    let out_map = temp_dir.path().join("var_cf_out.scx");
    let report = session.compile_map(&out_map).expect("should compile .scx map");
    assert_eq!(report.byte_buf_count, 3);
    assert!(out_map.exists());
}

#[test]
fn inverts_simple_if_else_branch_conditions_in_place_to_minimize_triggers() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("invert_branch_test");
    let mode = hir.add_var("mode", 0);
    let out_val = hir.add_var("out_val", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));

    // Since then_body only mutates `out_val` (not `mode`) and `mode >= 10` is invertible to `mode <= 9`,
    // this entire if/else lowers to 2 triggers with 0 self-modifying NextPtr triggers!
    main_fn.block_mut(entry).push_stmt(HirStmt::If {
        conditions: vec![StockCondition::VarCmp {
            var: mode,
            comparison: Comparison::AtLeast,
            value: 10,
            mask: None,
        }],
        then_body: vec![HirStmt::VarAssignConst {
            var: out_val,
            value: 100,
        }],
        else_body: vec![HirStmt::VarAssignConst {
            var: out_val,
            value: 200,
        }],
    });
    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);

    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let lir = session.lower_to_lir().unwrap();
    let triggers: Vec<_> = lir
        .objects()
        .iter()
        .filter_map(|o| match o {
            LirObject::Trigger(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggers.len(),
        2,
        "invertible leaf if/else must lower to 2 triggers in-place"
    );
    assert_eq!(triggers[0].conditions[0].comparison, Comparison::AtLeast.encode());
    assert_eq!(triggers[1].conditions[0].comparison, Comparison::AtMost.encode());
}

