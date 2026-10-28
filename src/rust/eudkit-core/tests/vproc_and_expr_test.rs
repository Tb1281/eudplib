use eudkit_core::allocator::{allocate_lir_module, vproc_buffer_occupation_map};
use eudkit_core::hir::{
    Comparison, HirBasicBlock, HirBinOp, HirExpr, HirFunction, HirModule, HirStmt, HirTerminator,
    Modifier, StockAction, StockCondition,
};
use eudkit_core::lir::{
    FieldRef, LirModule, LirObject, LirTrigger, LirVProcBuffer, LirVProcSlot, VProcSlotField,
    ValueExpr,
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
fn allocates_unified_72_byte_stride_vproc_buffer_and_stacks_objects_into_gaps() {
    let slots = vec![
        LirVProcSlot {
            label: Some("v0".to_string()),
            mask: ValueExpr::Const(-1),
            dest: ValueExpr::Const(0),
            value: ValueExpr::Const(111),
            modifier: Modifier::SetTo,
            next_ptr: ValueExpr::Const(0),
        },
        LirVProcSlot {
            label: Some("v1".to_string()),
            mask: ValueExpr::Const(0x00FF_00FFu32 as i32),
            dest: ValueExpr::ObjectEpd {
                object: eudkit_core::lir::ObjectId(2),
                dword_offset: 0,
            },
            value: ValueExpr::Const(222),
            modifier: Modifier::Add,
            next_ptr: ValueExpr::ObjectAddress {
                object: eudkit_core::lir::ObjectId(0),
                byte_offset: 0,
            },
        },
    ];

    // Check occupation map for 2 slots: 2404 + 72 * (2 - 1) = 2476 bytes = 619 dwords
    let occup = vproc_buffer_occupation_map(2);
    assert_eq!(occup.len() * 4, 2404 + 72);
    assert!(
        occup.iter().any(|&x| x == -1),
        "VProcBuffer occupation map must have unoccupied gaps (-1) for object stacking"
    );

    let entry_trig = LirTrigger {
        id: eudkit_core::lir::ObjectId(0),
        label: Some("entry".to_string()),
        prev_ptr: ValueExpr::Const(0),
        next_ptr: ValueExpr::FieldAddress(FieldRef::vproc_slot(1, 0, VProcSlotField::TriggerBase)),
        conditions: Vec::new(),
        actions: Vec::new(),
        flags: 4,
    };
    let vproc_buf = LirVProcBuffer {
        id: eudkit_core::lir::ObjectId(1),
        slots,
    };
    let cell_buf = eudkit_core::lir::LirByteBuf {
        id: eudkit_core::lir::ObjectId(2),
        label: Some("cell".to_string()),
        data: 0x1234_5678u32.to_le_bytes().to_vec(),
    };

    let lir = LirModule::new(
        "vproc_alloc_test",
        vec![
            LirObject::Trigger(entry_trig),
            LirObject::VProcBuffer(vproc_buf),
            LirObject::ByteBuf(cell_buf),
        ],
        eudkit_core::lir::ObjectId(0),
    );
    assert_eq!(lir.vproc_count(), 2);

    let allocated = allocate_lir_module(&lir).expect("should allocate LIR with VProcBuffer");
    // Unstacked size would be 2408 + 2476 + 4 = 4888 bytes; object stacking must pack them tighter!
    assert!(
        allocated.data.len() < 2408 + 2476,
        "VProcBuffer and Trigger must stack into overlapping unused gaps (got {} bytes)",
        allocated.data.len()
    );

    let vbuf_base = allocated.object_offsets[1] as usize;
    // Slot 0 value is at vbuf_base + 324 + 20 = vbuf_base + 344
    let v0_val = i32::from_le_bytes(
        allocated.data[vbuf_base + 344..vbuf_base + 348]
            .try_into()
            .unwrap(),
    );
    assert_eq!(v0_val, 111);

    // Slot 1 value is at vbuf_base + 324 + 72 + 20 = vbuf_base + 416
    let v1_val = i32::from_le_bytes(
        allocated.data[vbuf_base + 416..vbuf_base + 420]
            .try_into()
            .unwrap(),
    );
    assert_eq!(v1_val, 222);

    // Slot 1 modifier is Add (8) at vbuf_base + 324 + 72 + 27
    assert_eq!(
        allocated.data[vbuf_base + 324 + 72 + 27],
        Modifier::Add.encode()
    );
    // Slot 1 next_ptr is at vbuf_base + 72 * 1 = vbuf_base + 72
    let v1_nptr = u32::from_le_bytes(
        allocated.data[vbuf_base + 72..vbuf_base + 76]
            .try_into()
            .unwrap(),
    );
    assert_eq!(v1_nptr, allocated.object_offsets[0]);
}

#[test]
fn partitions_constant_only_and_dynamic_variables_and_reuses_rvalue_scratch_slots() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("storage_and_expr_test");
    let const_only_1 = hir.add_var("const_only_1", 10);
    let const_only_2 = hir.add_var("const_only_2", 20);
    let dyn_a = hir.add_var("dyn_a", 5);
    let dyn_b = hir.add_var("dyn_b", 7);
    let dyn_c = hir.add_var("dyn_c", 3);
    let out_d = hir.add_var("out_d", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));

    // Constant-only operations on const_only_1 and const_only_2
    main_fn.block_mut(entry).push_stmt(HirStmt::VarAddConst {
        var: const_only_1,
        value: 5,
    });
    main_fn.block_mut(entry).push_stmt(HirStmt::VarAssignExpr {
        var: const_only_2,
        // Pure constant folding: (10 + 20 * 3) ^ 0x0F -> folds to constant 70 ^ 15 = 73!
        // Because it folds to a constant in HIR, const_only_2 needs 0 VProc slots!
        expr: HirExpr::BinOp {
            op: HirBinOp::BitXor,
            lhs: Box::new(HirExpr::BinOp {
                op: HirBinOp::Add,
                lhs: Box::new(HirExpr::Const(10)),
                rhs: Box::new(HirExpr::BinOp {
                    op: HirBinOp::Mul,
                    lhs: Box::new(HirExpr::Const(20)),
                    rhs: Box::new(HirExpr::Const(3)),
                }),
            }),
            rhs: Box::new(HirExpr::Const(0x0F)),
        },
    });

    // Compound expression with 5 binary ops:
    // out_d = (dyn_a + dyn_b) + (dyn_a - dyn_c) + (dyn_b & 0xFF)
    // Legacy eudplib allocates a new EUDVariable per intermediate binary op;
    // eudkit must reuse its scratch pool across statements/subexpressions!
    let expr1 = HirExpr::BinOp {
        op: HirBinOp::Add,
        lhs: Box::new(HirExpr::BinOp {
            op: HirBinOp::Add,
            lhs: Box::new(HirExpr::BinOp {
                op: HirBinOp::Add,
                lhs: Box::new(HirExpr::Var(dyn_a)),
                rhs: Box::new(HirExpr::Var(dyn_b)),
            }),
            rhs: Box::new(HirExpr::BinOp {
                op: HirBinOp::Sub,
                lhs: Box::new(HirExpr::Var(dyn_a)),
                rhs: Box::new(HirExpr::Var(dyn_c)),
            }),
        }),
        rhs: Box::new(HirExpr::BinOp {
            op: HirBinOp::BitAnd,
            lhs: Box::new(HirExpr::Var(dyn_b)),
            rhs: Box::new(HirExpr::Const(0xFF)),
        }),
    };
    main_fn.block_mut(entry).push_stmt(HirStmt::VarAssignExpr {
        var: out_d,
        expr: expr1,
    });

    // Second compound expression in the same function:
    // out_d = (dyn_a << 2) ^ (dyn_b | dyn_c)
    // Should reuse the exact same scratch VProc slot(s) from the first expression!
    let expr2 = HirExpr::BinOp {
        op: HirBinOp::BitXor,
        lhs: Box::new(HirExpr::BinOp {
            op: HirBinOp::Shl,
            lhs: Box::new(HirExpr::Var(dyn_a)),
            rhs: Box::new(HirExpr::Const(2)),
        }),
        rhs: Box::new(HirExpr::BinOp {
            op: HirBinOp::BitOr,
            lhs: Box::new(HirExpr::Var(dyn_b)),
            rhs: Box::new(HirExpr::Var(dyn_c)),
        }),
    };
    main_fn.block_mut(entry).push_stmt(HirStmt::VarAssignExpr {
        var: out_d,
        expr: expr2,
    });

    // Dynamic comparison in If: if dyn_a >= dyn_b
    main_fn.block_mut(entry).push_stmt(HirStmt::If {
        conditions: vec![StockCondition::ExprCmp {
            lhs: HirExpr::Var(dyn_a),
            comparison: Comparison::AtLeast,
            rhs: HirExpr::Var(dyn_b),
        }],
        then_body: vec![HirStmt::VarAddConst {
            var: const_only_1,
            value: 1,
        }],
        else_body: Vec::new(),
    });

    // Dynamic condition/action arguments (Deaths with dynamic player & amount, SetDeaths with dynamic player & value)
    main_fn.block_mut(entry).push_stmt(HirStmt::If {
        conditions: vec![StockCondition::DeathsDynamic {
            player: HirExpr::Var(dyn_a),
            comparison: Comparison::Exactly,
            amount: HirExpr::Var(dyn_b),
            unit: 0,
            mask: None,
        }],
        then_body: vec![HirStmt::Action(StockAction::SetDeathsDynamic {
            player: HirExpr::Var(dyn_a),
            modifier: Modifier::SetTo,
            value: HirExpr::Var(dyn_c),
            unit: 0,
            mask: None,
        })],
        else_body: Vec::new(),
    });

    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);
    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let lir = session.lower_to_lir().expect("should lower to LIR").clone();

    // Verify const_only_1, const_only_2, and out_d (which is only written to, never read dynamically!)
    // lowered to 4-byte ByteBuf cells!
    let byte_buf_labels: Vec<String> = lir
        .objects()
        .iter()
        .filter_map(|o| match o {
            LirObject::ByteBuf(b) => b.label.clone(),
            _ => None,
        })
        .collect();
    assert!(
        byte_buf_labels.contains(&"var_const_only_1".to_string()),
        "const_only_1 must stay in a 4-byte ByteBuf cell"
    );
    assert!(
        byte_buf_labels.contains(&"var_const_only_2".to_string()),
        "const_only_2 (assigned constant-folded expr) must stay in a 4-byte ByteBuf cell"
    );
    assert!(
        byte_buf_labels.contains(&"var_out_d".to_string()),
        "out_d (only written to, never dynamically read) must stay in a 4-byte ByteBuf cell"
    );

    // Verify dyn_a, dyn_b, dyn_c + at most 2 scratch vars are in the unified VProcBuffer!
    assert!(
        lir.vproc_count() >= 3 && lir.vproc_count() <= 5,
        "expected 3 dynamic vars + 1..2 reused scratch VProc slots, got {}",
        lir.vproc_count()
    );
}

#[test]
fn lowers_dynamic_mul_div_rem_and_variable_shifts() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("dyn_muldiv_shift_test");
    let a = hir.add_var("a", 120);
    let b = hir.add_var("b", 7);
    let c = hir.add_var("c", 3);
    let out = hir.add_var("out", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));

    for op in [
        HirBinOp::Mul,
        HirBinOp::Div,
        HirBinOp::Rem,
        HirBinOp::Shl,
        HirBinOp::Shr,
    ] {
        main_fn.block_mut(entry).push_stmt(HirStmt::VarAssignExpr {
            var: out,
            expr: HirExpr::BinOp {
                op,
                lhs: Box::new(HirExpr::Var(a)),
                rhs: Box::new(if matches!(op, HirBinOp::Shl | HirBinOp::Shr) {
                    HirExpr::Var(c)
                } else {
                    HirExpr::Var(b)
                }),
            },
        });
    }

    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);
    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let lir = session
        .lower_to_lir()
        .expect("should lower dynamic mul/div/rem/shl/shr");
    let alloc = allocate_lir_module(lir).expect("should allocate payload");
    assert!(alloc.data.len() > 2400);
}
