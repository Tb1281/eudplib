use eudkit_core::hir::{
    epd, Comparison, HirBasicBlock, HirExpr, HirFunction, HirModule, HirStmt, HirTerminator,
    StockCondition,
};
use eudkit_core::lir::{FieldRef, LirModule, LirObject, ObjectId, TriggerRole, ValueExpr};
use eudkit_core::{allocate_lir_module, CompilationConfig, CompilationSession};
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

fn label_test_hir() -> HirModule {
    let mut hir = HirModule::new("lir_label_independence");
    let v = hir.add_var("v", 0);
    let dst = hir.add_var("dst", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));
    main_fn.block_mut(entry).push_stmt(HirStmt::If {
        // Two dynamic operands produce two PatchCondition triggers.
        conditions: vec![StockCondition::DeathsDynamic {
            player: HirExpr::Var(v),
            comparison: Comparison::AtLeast,
            amount: HirExpr::Var(v),
            unit: 0,
            mask: None,
        }],
        then_body: vec![HirStmt::MemRead {
            dst,
            epd: HirExpr::Const(epd(0x59CCA8 + 0x20)),
            // An arbitrary mask that only the read-table `(shift, msb)` map
            // can resolve; no exact `entries` key exists for it.
            mask: 0x0000_0F00,
            shift: -8,
            initial_val: 0,
        }],
        else_body: Vec::new(),
    });
    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);
    hir.add_function(main_fn);
    hir
}

fn strip_labels(module: &mut LirModule) {
    for object in module.objects_mut() {
        match object {
            LirObject::Trigger(trigger) => trigger.label = None,
            LirObject::VProcBuffer(buffer) => {
                for slot in &mut buffer.slots {
                    slot.label = None;
                }
            }
            LirObject::ByteBuf(buffer) => buffer.label = None,
        }
    }
}

fn trigger_ids_and_roles(module: &LirModule) -> Vec<(ObjectId, TriggerRole)> {
    module
        .objects()
        .iter()
        .filter_map(|object| match object {
            LirObject::Trigger(trigger) => Some((trigger.id, trigger.role)),
            _ => None,
        })
        .collect()
}

fn field_epd_objects(module: &LirModule) -> Vec<ObjectId> {
    let mut objects = Vec::new();
    for object in module.objects() {
        let LirObject::Trigger(trigger) = object else {
            continue;
        };
        for action in &trigger.actions {
            if let ValueExpr::FieldEpd(FieldRef { object, .. }) = action.player2 {
                objects.push(object);
            }
        }
    }
    objects
}

#[test]
fn stripping_all_labels_preserves_trigger_graph_and_payload() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    session.set_hir_module(label_test_hir());

    let labeled = session
        .lower_to_lir()
        .expect("should lower HIR with dynamic patches and MemRead")
        .clone();

    let patch_sites = labeled
        .objects()
        .iter()
        .filter_map(|object| match object {
            LirObject::Trigger(trigger) if trigger.role.is_patch() => Some(trigger),
            _ => None,
        })
        .count();
    assert!(
        patch_sites >= 2,
        "dynamic condition operands must produce typed patch triggers"
    );
    assert!(
        labeled
            .objects()
            .iter()
            .any(|object| matches!(object, LirObject::Trigger(t) if t.label.is_some())),
        "labels stay populated for LIR dumps"
    );
    for object in labeled.objects() {
        if let LirObject::Trigger(trigger) = object {
            for action in &trigger.actions {
                if let ValueExpr::FieldEpd(FieldRef { object, .. }) = &action.player2 {
                    assert_ne!(
                        *object,
                        ObjectId(u32::MAX),
                        "patch target must be resolved during emission"
                    );
                }
            }
        }
    }

    let mut unlabeled = labeled.clone();
    strip_labels(&mut unlabeled);

    assert_eq!(labeled.trigger_count(), unlabeled.trigger_count());
    assert_eq!(
        trigger_ids_and_roles(&labeled),
        trigger_ids_and_roles(&unlabeled),
        "trigger ids and typed roles must not depend on labels"
    );
    assert_eq!(
        field_epd_objects(&labeled),
        field_epd_objects(&unlabeled),
        "patch targets and other FieldRefs must not depend on labels"
    );

    let labeled_payload = allocate_lir_module(&labeled).expect("labeled LIR must allocate");
    let unlabeled_payload = allocate_lir_module(&unlabeled).expect("unlabeled LIR must allocate");
    assert_eq!(labeled_payload.data, unlabeled_payload.data);
    assert_eq!(
        labeled_payload.object_offsets,
        unlabeled_payload.object_offsets
    );
    assert_eq!(labeled_payload.entry_offset, unlabeled_payload.entry_offset);
    assert_eq!(labeled_payload.prt_table, unlabeled_payload.prt_table);
    assert_eq!(labeled_payload.ort_table, unlabeled_payload.ort_table);
}
