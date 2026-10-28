use eudkit_core::hir::{
    Comparison, HirBasicBlock, HirByteBuf, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, Modifier, PlayerId, ResourceType, StockAction, StockCondition,
};
use eudkit_core::lir::{ActField, CondField, FieldRef, LirObject, TriggerSlotField, ValueExpr};
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
fn lowers_hir_module_with_field_refs_and_dumps_deterministic_eudir_snapshots() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("tracer_bullet");
    let buf_id = hir.add_byte_buf(HirByteBuf::new("counter_cell", vec![0, 0, 0, 0]));

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry_block = main_fn.add_block(HirBasicBlock::new("entry"));
    let then_block = main_fn.add_block(HirBasicBlock::new("on_death_match"));
    let exit_block = main_fn.add_block(HirBasicBlock::new("exit"));

    // Symbolic FieldRef targeting trigger 0's Action[0].Player2 (value slot) and Condition[0].Amount
    let self_mod_act_ref = FieldRef::action(0, 0, ActField::Player2);
    let self_mod_cond_ref = FieldRef::condition(0, 0, CondField::Amount);
    let self_mod_next_ref = FieldRef::next_ptr(0);

    assert_eq!(self_mod_next_ref.byte_offset(), 4);
    assert_eq!(self_mod_cond_ref.byte_offset(), 8 + 8);
    assert_eq!(self_mod_act_ref.byte_offset(), 328 + 20);

    main_fn.block_mut(entry_block).push_stmt(HirStmt::Trigger {
        label: Some("init_step".to_string()),
        conditions: vec![StockCondition::Deaths {
            player: PlayerId::Player1,
            comparison: Comparison::AtLeast,
            amount: 0,
            unit: 0,
        }],
        actions: vec![
            StockAction::SetResources {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                amount: 50,
                resource: ResourceType::Ore,
            },
            StockAction::DisplayText {
                text: "Hello from eudkit tracer bullet!".to_string(),
                always_display: true,
            },
            StockAction::SetField {
                target: self_mod_act_ref,
                modifier: Modifier::Add,
                value: HirValue::Const(1),
            },
            StockAction::SetDeaths {
                player: PlayerId::CurrentPlayer,
                modifier: Modifier::SetTo,
                value: HirValue::ByteBufAddress(buf_id, 0),
                unit: 0,
            },
        ],
        preserved: true,
    });
    main_fn
        .block_mut(entry_block)
        .set_terminator(HirTerminator::Branch {
            conditions: vec![StockCondition::Deaths {
                player: PlayerId::Player1,
                comparison: Comparison::Exactly,
                amount: 1,
                unit: 0,
            }],
            then_block,
            else_block: exit_block,
        });

    main_fn
        .block_mut(then_block)
        .push_stmt(HirStmt::Action(StockAction::SetDeaths {
            player: PlayerId::Player1,
            modifier: Modifier::Add,
            value: HirValue::Const(10),
            unit: 0,
        }));
    main_fn
        .block_mut(then_block)
        .set_terminator(HirTerminator::Jump(exit_block));

    main_fn
        .block_mut(exit_block)
        .set_terminator(HirTerminator::Return);

    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let hir_dump = session.dump_hir();
    assert!(hir_dump.contains("module @tracer_bullet"));
    assert!(hir_dump.contains("bytebuf @counter_cell"));
    assert!(hir_dump.contains("fn @main (main_loop)"));
    assert!(hir_dump.contains("DisplayText(\"Hello from eudkit tracer bullet!\")"));
    assert!(hir_dump.contains("SetField(@obj0.Action[0].Player2"));

    let lir = session
        .lower_to_lir()
        .expect("should lower HIR to LIR")
        .clone();
    let lir_dump = session.dump_lir().expect("should dump LIR");

    assert!(lir_dump.contains("lir_module @tracer_bullet"));
    assert!(lir_dump.contains("object @obj"));
    assert!(lir_dump.contains("NextPtr"));
    assert_eq!(
        lir_dump,
        session.dump_lir().unwrap(),
        "LIR dump must be deterministic"
    );

    // Verify LIR contains both ByteBuf and Trigger objects and preserves FieldRef expressions
    let has_byte_buf = lir
        .objects()
        .iter()
        .any(|o| matches!(o, LirObject::ByteBuf(b) if b.data.len() == 4));
    assert!(
        has_byte_buf,
        "LIR should contain 4-byte counter_cell ByteBuf"
    );

    let has_field_epd = lir.objects().iter().any(|o| {
        if let LirObject::Trigger(t) = o {
            t.actions.iter().any(|a| {
                matches!(
                    a.player1,
                    ValueExpr::FieldEpd(FieldRef {
                        field: TriggerSlotField::Action(0, ActField::Player2)
                            | TriggerSlotField::NextPtr,
                        ..
                    })
                )
            })
        } else {
            false
        }
    });
    assert!(
        has_field_epd,
        "LIR should use symbolic FieldRef for self-modifying actions"
    );
}
