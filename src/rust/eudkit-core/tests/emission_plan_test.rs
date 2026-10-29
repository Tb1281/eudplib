use eudkit_core::hir::{
    BlockId, FuncId, FuncPtrId, HirByteBuf, HirExpr, HirFunction, HirModule, Modifier, StockAction,
    StockCondition, VarId,
};
use eudkit_core::lir::storage_selection::VarStorage;
use eudkit_core::lir::trigger_shape::{
    block_trigger_shape, BlockTriggerShape, DYNAMIC_DIV_CHAIN_TRIGGERS, DYNAMIC_MUL_TRIGGERS,
    DYNAMIC_SHR_TRIGGERS,
};
use eudkit_core::lir::{
    emit_lir, ActField, CondField, EmissionPlan, ExpandedBlock, ExpandedCfg, ExpandedOp,
    ExpandedTerminator, LirObject, LirTrigger, ObjectId,
};
use eudkit_core::{CompilationConfig, CompilationSession};
use std::collections::HashMap;
use std::path::PathBuf;

fn block(id: u32, ops: Vec<ExpandedOp>, terminator: ExpandedTerminator) -> ExpandedBlock {
    ExpandedBlock {
        id: BlockId(id),
        label: format!("block_{id}"),
        ops,
        terminator,
    }
}

fn shape_of(op: ExpandedOp) -> BlockTriggerShape {
    block_trigger_shape(&block(0, vec![op], ExpandedTerminator::Return))
}

fn comment(text: &str) -> ExpandedOp {
    ExpandedOp::LeafAction(StockAction::Comment {
        text: text.to_string(),
    })
}

#[test]
fn trigger_shape_policy_owns_representative_op_spans() {
    let dynamic_mul = shape_of(ExpandedOp::DynamicMul {
        lhs: VarId(0),
        rhs: VarId(1),
        dst: VarId(2),
    });
    assert_eq!(dynamic_mul.op_span(0).trigger_count, DYNAMIC_MUL_TRIGGERS);
    assert_eq!(dynamic_mul.op_span(0).pending_actions_after, 0);
    assert_eq!(dynamic_mul.trigger_count, DYNAMIC_MUL_TRIGGERS);
    assert_eq!(dynamic_mul.terminator.trigger_count, 0);

    let dynamic_div = shape_of(ExpandedOp::DynamicDivRem {
        dividend: VarId(0),
        divisor: VarId(1),
        quot_dst: VarId(2),
        div_x_scratch: VarId(3),
    });
    assert_eq!(
        dynamic_div.op_span(0).trigger_count,
        DYNAMIC_DIV_CHAIN_TRIGGERS
    );
    assert_eq!(dynamic_div.trigger_count, DYNAMIC_DIV_CHAIN_TRIGGERS);

    let dynamic_shr = shape_of(ExpandedOp::DynamicShr {
        dst: VarId(0),
        shift: VarId(1),
        pow2_scratch: VarId(2),
        div_x_scratch: VarId(3),
        div_q_scratch: VarId(4),
    });
    assert_eq!(dynamic_shr.op_span(0).trigger_count, DYNAMIC_SHR_TRIGGERS);
    assert_eq!(dynamic_shr.trigger_count, DYNAMIC_SHR_TRIGGERS);

    let dynamic_shl = shape_of(ExpandedOp::DynamicShl {
        dst: VarId(0),
        shift: VarId(1),
    });
    assert_eq!(dynamic_shl.op_span(0).trigger_count, 3);
    assert_eq!(dynamic_shl.op_span(0).pending_actions_after, 1);
    assert_eq!(dynamic_shl.terminator.trigger_count, 1);
    assert_eq!(dynamic_shl.trigger_count, 4);

    let vproc_copy_masked = shape_of(ExpandedOp::VProcCopyVar {
        src: VarId(0),
        dst: VarId(1),
        modifier: Modifier::Add,
        mask: Some(0xFF),
    });
    assert_eq!(vproc_copy_masked.op_span(0).trigger_count, 1);
    assert_eq!(vproc_copy_masked.op_span(0).pending_actions_after, 1);
    assert_eq!(vproc_copy_masked.trigger_count, 2);

    let vproc_copy = shape_of(ExpandedOp::VProcCopyVar {
        src: VarId(0),
        dst: VarId(1),
        modifier: Modifier::Add,
        mask: None,
    });
    assert_eq!(vproc_copy.op_span(0).trigger_count, 1);
    assert_eq!(vproc_copy.op_span(0).pending_actions_after, 0);
    assert_eq!(vproc_copy.trigger_count, 1);

    for op in [
        ExpandedOp::VProcBitAndVar {
            src: VarId(0),
            dst: VarId(1),
        },
        ExpandedOp::VProcBitOrVar {
            src: VarId(0),
            dst: VarId(1),
        },
    ] {
        let shape = shape_of(op);
        assert_eq!(shape.op_span(0).trigger_count, 2);
        assert_eq!(shape.trigger_count, 2);
    }

    let vproc_xor = shape_of(ExpandedOp::VProcBitXorVar {
        src: VarId(0),
        dst: VarId(1),
    });
    assert_eq!(vproc_xor.op_span(0).trigger_count, 2);
    assert_eq!(vproc_xor.op_span(0).pending_actions_after, 1);
    assert_eq!(vproc_xor.trigger_count, 3);

    let patch_cond = shape_of(ExpandedOp::PatchNextTriggerCond {
        src: VarId(0),
        cond_idx: 0,
        field: CondField::Amount,
        is_epd: false,
    });
    assert_eq!(patch_cond.op_span(0).trigger_count, 1);
    assert_eq!(patch_cond.op_span(0).patch_sites, 1);
    assert_eq!(patch_cond.patch_sites, 1);

    let patch_act = shape_of(ExpandedOp::PatchNextTriggerAct {
        src: VarId(0),
        act_idx: 0,
        field: ActField::Player2,
        is_epd: true,
    });
    assert_eq!(patch_act.op_span(0).trigger_count, 1);
    assert_eq!(patch_act.op_span(0).patch_sites, 1);
    assert_eq!(patch_act.patch_sites, 1);

    let call = shape_of(ExpandedOp::CallSetup {
        callee: FuncId(1),
        rets: vec![VarId(0), VarId(1)],
        return_block: BlockId(1),
    });
    assert_eq!(call.op_span(0).trigger_count, 0);
    assert_eq!(call.op_span(0).pending_actions_after, 3);
    assert_eq!(call.trigger_count, 1);

    let call_void = shape_of(ExpandedOp::CallSetup {
        callee: FuncId(1),
        rets: Vec::new(),
        return_block: BlockId(1),
    });
    assert_eq!(call_void.op_span(0).pending_actions_after, 1);

    let write_slot_src = shape_of(ExpandedOp::WriteFuncReturnSlot {
        func: FuncId(1),
        slot_idx: 0,
        src_var: Some(VarId(0)),
        const_val: None,
    });
    assert_eq!(write_slot_src.op_span(0).trigger_count, 1);
    assert_eq!(write_slot_src.trigger_count, 1);

    let write_slot_const = shape_of(ExpandedOp::WriteFuncReturnSlot {
        func: FuncId(1),
        slot_idx: 0,
        src_var: None,
        const_val: Some(7),
    });
    assert_eq!(write_slot_const.op_span(0).trigger_count, 0);
    assert_eq!(write_slot_const.op_span(0).pending_actions_after, 1);

    let assign_target = shape_of(ExpandedOp::FuncPtrAssignTarget {
        ptr: FuncPtrId(0),
        target: FuncId(1),
    });
    assert_eq!(assign_target.op_span(0).pending_actions_after, 1);

    let mem_read_const = shape_of(ExpandedOp::MemReadSetup {
        dst: VarId(0),
        epd: HirExpr::Const(0x1000),
        mask: 0xFF,
        shift: 0,
        initial_val: 0,
        return_block: BlockId(1),
    });
    assert_eq!(mem_read_const.op_span(0).pending_actions_after, 3);

    let mem_read_const_initial = shape_of(ExpandedOp::MemReadSetup {
        dst: VarId(0),
        epd: HirExpr::Const(0x1000),
        mask: 0xFF,
        shift: 0,
        initial_val: 5,
        return_block: BlockId(1),
    });
    assert_eq!(mem_read_const_initial.op_span(0).pending_actions_after, 4);

    let mem_read_var = shape_of(ExpandedOp::MemReadSetup {
        dst: VarId(0),
        epd: HirExpr::Var(VarId(1)),
        mask: 0xFF,
        shift: 0,
        initial_val: 0,
        return_block: BlockId(1),
    });
    assert_eq!(mem_read_var.op_span(0).trigger_count, 1);
    assert_eq!(mem_read_var.trigger_count, 1);

    let leaf = shape_of(comment("leaf"));
    assert_eq!(leaf.op_span(0).trigger_count, 0);
    assert_eq!(leaf.op_span(0).pending_actions_after, 1);
    assert_eq!(leaf.trigger_count, 1);

    let trigger = shape_of(ExpandedOp::Trigger {
        label: None,
        conditions: Vec::new(),
        actions: Vec::new(),
        preserved: false,
    });
    assert_eq!(trigger.op_span(0).trigger_count, 1);
    assert_eq!(trigger.trigger_count, 1);
}

#[test]
fn trigger_shape_policy_coalesces_at_the_sixty_four_action_boundary() {
    let leaves =
        |n: usize| -> Vec<ExpandedOp> { (0..n).map(|i| comment(&format!("leaf_{i}"))).collect() };

    let shape_64 = block_trigger_shape(&block(0, leaves(64), ExpandedTerminator::Return));
    assert_eq!(shape_64.trigger_count, 1);
    assert_eq!(shape_64.op_spans()[63].trigger_count, 1);
    assert_eq!(shape_64.op_spans()[63].pending_actions_after, 0);
    assert!(shape_64.op_spans()[..63]
        .iter()
        .all(|span| span.trigger_count == 0));
    assert_eq!(shape_64.terminator.trigger_count, 0);

    let shape_65 = block_trigger_shape(&block(0, leaves(65), ExpandedTerminator::Return));
    assert_eq!(shape_65.trigger_count, 2);
    assert_eq!(shape_65.op_spans()[63].trigger_count, 1);
    assert_eq!(shape_65.op_spans()[64].trigger_count, 0);
    assert_eq!(shape_65.op_spans()[64].pending_actions_after, 1);
    assert_eq!(shape_65.terminator.trigger_count, 1);

    let mut overflow_ops = leaves(63);
    overflow_ops.push(ExpandedOp::VProcCopyVar {
        src: VarId(0),
        dst: VarId(1),
        modifier: Modifier::Add,
        mask: None,
    });
    let overflow = block_trigger_shape(&block(0, overflow_ops, ExpandedTerminator::Return));
    assert_eq!(overflow.op_spans()[62].pending_actions_after, 63);
    assert_eq!(overflow.op_spans()[63].trigger_count, 2);
    assert_eq!(overflow.trigger_count, 2);
}

#[test]
fn empty_block_shapes_reserve_the_terminator_trigger() {
    let return_shape = block_trigger_shape(&block(0, Vec::new(), ExpandedTerminator::Return));
    assert_eq!(return_shape.trigger_count, 1);
    assert_eq!(return_shape.terminator.trigger_count, 1);

    let jump_shape =
        block_trigger_shape(&block(0, Vec::new(), ExpandedTerminator::Jump(BlockId(1))));
    assert_eq!(jump_shape.trigger_count, 1);
    assert_eq!(jump_shape.terminator.trigger_count, 1);

    let branch_shape = block_trigger_shape(&block(
        0,
        Vec::new(),
        ExpandedTerminator::Branch {
            conditions: Vec::new(),
            then_block: BlockId(1),
            else_block: BlockId(2),
        },
    ));
    assert_eq!(branch_shape.trigger_count, 2);
    assert_eq!(branch_shape.terminator.trigger_count, 2);
}

fn agreement_fixture() -> (HirModule, ExpandedCfg, FuncId) {
    let mut hir = HirModule::new("trigger_shape_agreement");
    let main_id = hir.add_function(HirFunction::new_main_loop("main"));
    let helper_id = hir.add_function(HirFunction::new("helper", false).with_return_count(2));
    let func_ptr = hir.add_func_ptr("callback", (0, 2), Some(helper_id));

    let v0 = hir.add_var("v0", 0);
    let v1 = hir.add_var("v1", 0);
    let v2 = hir.add_var("v2", 0);
    let v3 = hir.add_var("v3", 0);
    let v4 = hir.add_var("v4", 0);
    let v5 = hir.add_var("v5", 0);
    let v6 = hir.add_var("v6", 0);

    let mut fn_entry_block = HashMap::new();
    fn_entry_block.insert(helper_id, BlockId(1));
    let mut fp_trampoline_block = HashMap::new();
    fp_trampoline_block.insert(func_ptr, BlockId(2));

    let ops = vec![
        comment("before target"),
        ExpandedOp::Trigger {
            label: None,
            conditions: vec![StockCondition::Always],
            actions: vec![StockAction::Comment {
                text: "patch target".to_string(),
            }],
            preserved: false,
        },
        ExpandedOp::PatchNextTriggerCond {
            src: v0,
            cond_idx: 0,
            field: CondField::Amount,
            is_epd: false,
        },
        ExpandedOp::PatchNextTriggerAct {
            src: v0,
            act_idx: 0,
            field: ActField::Player2,
            is_epd: true,
        },
        ExpandedOp::Trigger {
            label: None,
            conditions: vec![StockCondition::Always],
            actions: vec![StockAction::Comment {
                text: "patch target".to_string(),
            }],
            preserved: false,
        },
        ExpandedOp::VProcCopyVar {
            src: v0,
            dst: v1,
            modifier: Modifier::Add,
            mask: Some(0xFF),
        },
        ExpandedOp::VProcCopyVar {
            src: v0,
            dst: v1,
            modifier: Modifier::SetTo,
            mask: None,
        },
        ExpandedOp::VProcBitAndVar { src: v0, dst: v1 },
        ExpandedOp::VProcBitOrVar { src: v0, dst: v1 },
        ExpandedOp::VProcBitXorVar { src: v0, dst: v1 },
        ExpandedOp::DynamicShl { dst: v2, shift: v3 },
        ExpandedOp::DynamicMul {
            lhs: v0,
            rhs: v1,
            dst: v2,
        },
        ExpandedOp::DynamicDivRem {
            dividend: v0,
            divisor: v1,
            quot_dst: v2,
            div_x_scratch: v5,
        },
        ExpandedOp::DynamicShr {
            dst: v2,
            shift: v3,
            pow2_scratch: v4,
            div_x_scratch: v5,
            div_q_scratch: v6,
        },
        ExpandedOp::CallSetup {
            callee: helper_id,
            rets: vec![v0, v1],
            return_block: BlockId(1),
        },
        ExpandedOp::CallIndirectSetup {
            ptr: func_ptr,
            rets: vec![v0],
            return_block: BlockId(1),
        },
        ExpandedOp::WriteFuncReturnSlot {
            func: helper_id,
            slot_idx: 0,
            src_var: Some(v0),
            const_val: None,
        },
        ExpandedOp::WriteFuncReturnSlot {
            func: helper_id,
            slot_idx: 1,
            src_var: None,
            const_val: Some(7),
        },
        ExpandedOp::FuncPtrAssignTarget {
            ptr: func_ptr,
            target: helper_id,
        },
        ExpandedOp::MemReadSetup {
            dst: v0,
            epd: HirExpr::Const(0x1000),
            mask: 0xFF,
            shift: 0,
            initial_val: 0,
            return_block: BlockId(1),
        },
        ExpandedOp::MemReadSetup {
            dst: v0,
            epd: HirExpr::Const(0x1000),
            mask: 0xFF,
            shift: 0,
            initial_val: 5,
            return_block: BlockId(1),
        },
        ExpandedOp::MemReadSetup {
            dst: v0,
            epd: HirExpr::Var(v1),
            mask: 0xFF,
            shift: 0,
            initial_val: 0,
            return_block: BlockId(1),
        },
        ExpandedOp::MemReadSetup {
            dst: v0,
            epd: HirExpr::Var(v1),
            mask: 0xFF,
            shift: 0,
            initial_val: 5,
            return_block: BlockId(1),
        },
    ];

    let blocks = vec![
        block(
            0,
            ops,
            ExpandedTerminator::Branch {
                conditions: vec![StockCondition::Always],
                then_block: BlockId(1),
                else_block: BlockId(2),
            },
        ),
        block(
            1,
            vec![comment("second")],
            ExpandedTerminator::Jump(BlockId(2)),
        ),
        block(2, Vec::new(), ExpandedTerminator::Return),
    ];

    (
        hir,
        ExpandedCfg {
            blocks,
            fn_entry_block,
            fp_trampoline_block,
        },
        main_id,
    )
}

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

fn triggers_in_span(
    lir: &eudkit_core::lir::LirModule,
    start: ObjectId,
    end: ObjectId,
) -> Vec<&LirTrigger> {
    lir.objects()
        .iter()
        .filter_map(|object| match object {
            LirObject::Trigger(trigger) if trigger.id.0 >= start.0 && trigger.id.0 < end.0 => {
                Some(trigger)
            }
            _ => None,
        })
        .collect()
}

fn is_patch_trigger(trigger: &LirTrigger) -> bool {
    trigger
        .label
        .as_deref()
        .is_some_and(|label| label.contains("_patch_cond_") || label.contains("_patch_act_"))
}

#[test]
fn plan_matches_emitted_lir_trigger_spans_patch_sites_and_object_ids() {
    let (hir, expanded, main_id) = agreement_fixture();
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let plan = EmissionPlan::build(&hir, &expanded, main_id);
    let lir = emit_lir(&hir, expanded, session.map_context_mut()).expect("emit LIR");

    assert_eq!(lir.trigger_count() as u32, plan.trigger_count());
    assert_eq!(lir.entry_trigger, plan.block(0).first_trigger);

    let mut object_ids: Vec<u32> = lir.objects().iter().map(|object| object.id().0).collect();
    object_ids.sort_unstable();
    assert_eq!(object_ids, (0..plan.next_object_id()).collect::<Vec<_>>());

    let mut expected_first_trigger = 0;
    for (block_index, block_plan) in plan.blocks().iter().enumerate() {
        assert_eq!(block_plan.first_trigger, ObjectId(expected_first_trigger));
        expected_first_trigger += block_plan.trigger_budget;

        let block_triggers =
            triggers_in_span(&lir, block_plan.first_trigger, block_plan.end_trigger());
        assert_eq!(
            block_triggers.len() as u32,
            block_plan.trigger_budget,
            "block {block_index} trigger count diverged from its plan"
        );
        let emitted_ids: Vec<u32> = block_triggers.iter().map(|trigger| trigger.id.0).collect();
        assert_eq!(
            emitted_ids,
            (block_plan.first_trigger.0..block_plan.end_trigger().0).collect::<Vec<_>>(),
            "block {block_index} did not emit contiguous trigger ids"
        );
        let emitted_patch_sites = block_triggers
            .iter()
            .filter(|trigger| is_patch_trigger(trigger))
            .count() as u32;
        assert_eq!(
            emitted_patch_sites, block_plan.patch_sites,
            "block {block_index} patch sites diverged from its plan"
        );

        for (op_index, op_span) in block_plan.op_spans().iter().enumerate() {
            let op_start = ObjectId(block_plan.first_trigger.0 + op_span.first_trigger);
            let op_end = ObjectId(block_plan.first_trigger.0 + op_span.end_trigger());
            let op_triggers = triggers_in_span(&lir, op_start, op_end);
            assert_eq!(
                op_triggers.len() as u32,
                op_span.trigger_count,
                "block {block_index} op {op_index} trigger span diverged from its plan"
            );
            let op_patch_sites = op_triggers
                .iter()
                .filter(|trigger| is_patch_trigger(trigger))
                .count() as u32;
            assert_eq!(
                op_patch_sites, op_span.patch_sites,
                "block {block_index} op {op_index} patch sites diverged from its plan"
            );
        }

        let terminator_span = block_plan.terminator_span();
        let terminator_start = ObjectId(block_plan.first_trigger.0 + terminator_span.first_trigger);
        let terminator_end = ObjectId(block_plan.first_trigger.0 + terminator_span.end_trigger());
        assert_eq!(
            triggers_in_span(&lir, terminator_start, terminator_end).len() as u32,
            terminator_span.trigger_count,
            "block {block_index} terminator span diverged from its plan"
        );
    }
    assert_eq!(plan.block_trigger_end(), expected_first_trigger);
}

#[test]
fn plan_places_void_returns_read_table_and_storage_without_overlap() {
    let mut hir = HirModule::new("layout_test");
    let main = HirFunction::new_main_loop("main");
    let main_id = hir.add_function(main);
    let mut helper = HirFunction::new("helper", false);
    helper.return_count = 0;
    let helper_id = hir.add_function(helper);
    hir.main_loop = Some(main_id);

    let cell = hir.add_var("cell", 7);
    let promoted = hir.add_var("promoted", 9);
    let buf = hir.add_byte_buf(HirByteBuf::new("table", vec![1, 2, 3, 4]));
    let func_ptr = hir.add_func_ptr("callback", (0, 2), Some(helper_id));

    let mut fn_entry_block = HashMap::new();
    fn_entry_block.insert(helper_id, BlockId(0));
    let blocks = vec![block(
        0,
        vec![
            ExpandedOp::DynamicMul {
                lhs: cell,
                rhs: promoted,
                dst: cell,
            },
            ExpandedOp::MemReadSetup {
                dst: cell,
                epd: HirExpr::Const(0x2000),
                mask: 0xFF,
                shift: 0,
                initial_val: 0,
                return_block: BlockId(0),
            },
        ],
        ExpandedTerminator::Return,
    )];
    let expanded = ExpandedCfg {
        blocks,
        fn_entry_block,
        fp_trampoline_block: HashMap::new(),
    };

    let plan = EmissionPlan::build(&hir, &expanded, main_id);
    let block_end = plan.block_trigger_end();
    let read_table_len = plan
        .read_table()
        .expect("read table should exist")
        .triggers
        .len() as u32;

    assert_eq!(block_end, plan.block(0).trigger_budget);
    assert_eq!(
        plan.void_return_trigger(helper_id),
        Some(ObjectId(block_end))
    );
    assert_eq!(plan.trigger_count(), block_end + 1 + read_table_len);

    let vbuf_id = plan.vproc_buffer().expect("vbuf should exist").id;
    assert_eq!(vbuf_id, ObjectId(plan.trigger_count()));
    assert_eq!(
        plan.byte_buf_ids()[&buf],
        ObjectId(plan.trigger_count() + 1)
    );
    assert_eq!(
        plan.var_storage()[&cell],
        VarStorage::ByteBuf(ObjectId(plan.trigger_count() + 2))
    );
    assert!(matches!(
        plan.var_storage()[&promoted],
        VarStorage::VProcSlot { vbuf_obj, .. } if vbuf_obj == vbuf_id
    ));
    assert_eq!(plan.next_object_id(), plan.trigger_count() + 3);

    let return_slots = plan
        .func_return_slots(helper_id)
        .expect("func pointer targets share return slots");
    assert_eq!(return_slots.len(), 2);
    assert_eq!(plan.func_ptr_return_slots(func_ptr).unwrap(), return_slots);

    let slots = &plan.vproc_buffer().unwrap().slots;
    assert_eq!(slots.len(), 3, "promoted var + two return slots");
    assert_eq!(
        slots[return_slots[0] as usize].label.as_deref(),
        Some("fp_callback_ret_0")
    );
    assert_eq!(
        slots[return_slots[1] as usize].label.as_deref(),
        Some("fp_callback_ret_1")
    );

    let planned = plan.into_planned_objects();
    assert_eq!(planned.entry_trigger, ObjectId(0));
    assert_eq!(planned.vproc_buffer.unwrap().id, vbuf_id);
    assert_eq!(planned.byte_bufs[0].data, vec![1, 2, 3, 4]);
    assert_eq!(planned.cell_bufs[0].data, 7i32.to_le_bytes().to_vec());
    assert_eq!(planned.read_table_triggers.len() as u32, read_table_len);
    assert_eq!(
        planned.void_return_triggers[&helper_id],
        ObjectId(block_end)
    );
}
