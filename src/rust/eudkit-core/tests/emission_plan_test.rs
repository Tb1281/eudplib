use eudkit_core::hir::{
    BlockId, FuncId, FuncPtrId, HirByteBuf, HirExpr, HirFunction, HirModule, Modifier, StockAction,
    VarId,
};
use eudkit_core::lir::emission_plan::count_block_triggers;
use eudkit_core::lir::storage_selection::VarStorage;
use eudkit_core::lir::{
    ActField, CondField, EmissionPlan, ExpandedBlock, ExpandedCfg, ExpandedOp, ExpandedTerminator,
    ObjectId,
};
use std::collections::HashMap;

fn block(id: u32, ops: Vec<ExpandedOp>, terminator: ExpandedTerminator) -> ExpandedBlock {
    ExpandedBlock {
        id: BlockId(id),
        label: format!("block_{id}"),
        ops,
        terminator,
    }
}

fn cfg(blocks: Vec<ExpandedBlock>) -> ExpandedCfg {
    ExpandedCfg {
        blocks,
        fn_entry_block: HashMap::new(),
        fp_trampoline_block: HashMap::new(),
    }
}

fn empty_hir() -> HirModule {
    HirModule::new("plan_test")
}

#[test]
fn production_budgets_match_the_count_oracle_for_every_op_shape() {
    let blocks = vec![
        block(
            0,
            vec![
                ExpandedOp::LeafAction(StockAction::Comment {
                    text: "leaf".to_string(),
                }),
                ExpandedOp::Trigger {
                    label: None,
                    conditions: Vec::new(),
                    actions: Vec::new(),
                    preserved: false,
                },
                ExpandedOp::VProcCopyVar {
                    src: VarId(0),
                    dst: VarId(1),
                    modifier: Modifier::Add,
                    mask: Some(0xFF),
                },
                ExpandedOp::VProcBitAndVar {
                    src: VarId(0),
                    dst: VarId(1),
                },
                ExpandedOp::VProcBitOrVar {
                    src: VarId(0),
                    dst: VarId(1),
                },
                ExpandedOp::VProcBitXorVar {
                    src: VarId(0),
                    dst: VarId(1),
                },
                ExpandedOp::DynamicShl {
                    dst: VarId(0),
                    shift: VarId(1),
                },
                ExpandedOp::DynamicShr {
                    dst: VarId(0),
                    shift: VarId(1),
                    pow2_scratch: VarId(2),
                    div_x_scratch: VarId(3),
                    div_q_scratch: VarId(4),
                },
                ExpandedOp::DynamicMul {
                    lhs: VarId(0),
                    rhs: VarId(1),
                    dst: VarId(2),
                },
                ExpandedOp::DynamicDivRem {
                    dividend: VarId(0),
                    divisor: VarId(1),
                    quot_dst: VarId(2),
                    div_x_scratch: VarId(3),
                },
                ExpandedOp::PatchNextTriggerCond {
                    src: VarId(0),
                    cond_idx: 1,
                    field: CondField::Amount,
                    is_epd: false,
                },
                ExpandedOp::PatchNextTriggerAct {
                    src: VarId(0),
                    act_idx: 2,
                    field: ActField::Player2,
                    is_epd: true,
                },
                ExpandedOp::CallSetup {
                    callee: FuncId(1),
                    rets: vec![VarId(0), VarId(1)],
                    return_block: BlockId(1),
                },
                ExpandedOp::WriteFuncReturnSlot {
                    func: FuncId(1),
                    slot_idx: 0,
                    src_var: Some(VarId(0)),
                    const_val: None,
                },
                ExpandedOp::FuncPtrAssignTarget {
                    ptr: FuncPtrId(0),
                    target: FuncId(1),
                },
                ExpandedOp::MemReadSetup {
                    dst: VarId(0),
                    epd: HirExpr::Const(0x1000),
                    mask: 0xFF,
                    shift: 0,
                    initial_val: 0,
                    return_block: BlockId(1),
                },
            ],
            ExpandedTerminator::Branch {
                conditions: Vec::new(),
                then_block: BlockId(1),
                else_block: BlockId(2),
            },
        ),
        block(
            1,
            vec![ExpandedOp::LeafAction(StockAction::Comment {
                text: "second".to_string(),
            })],
            ExpandedTerminator::Jump(BlockId(2)),
        ),
        block(2, Vec::new(), ExpandedTerminator::Return),
    ];

    let plan = EmissionPlan::build(&empty_hir(), &cfg(blocks.clone()), FuncId(0));

    let mut expected_first = 0;
    for (planned, expanded) in plan.blocks().iter().zip(&blocks) {
        assert_eq!(
            planned.trigger_budget,
            count_block_triggers(expanded),
            "budget mismatch for {}",
            expanded.label
        );
        assert_eq!(planned.first_trigger, ObjectId(expected_first));
        expected_first += planned.trigger_budget;
    }
    assert_eq!(plan.block_trigger_end(), expected_first);
    let read_table_len = plan
        .read_table()
        .expect("read table should exist")
        .triggers
        .len() as u32;
    assert_eq!(plan.trigger_count(), expected_first + read_table_len);
    assert_eq!(plan.block(0).patch_sites, 2);
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
