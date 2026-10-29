use eudkit_core::hir::{Comparison, HirBuilder, HirModule, HirStmt, StockCondition, VarId};

#[test]
fn test_reactive_rules_builder_and_lowering() {
    let mut builder = HirBuilder::new("test_reactive");
    let test_var = builder.add_var("test_var", 0);

    // 1. Once rule
    builder.begin_once(vec![]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 10,
    });
    builder.end_reactive().expect("end_reactive once");

    // 2. When rule
    let cond = StockCondition::VarCmp {
        var: test_var,
        comparison: Comparison::AtLeast,
        value: 5,
        mask: None,
    };
    builder.begin_when(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 1,
    });
    builder.end_reactive().expect("end_reactive when");

    // 3. Until rule
    builder.begin_until(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 2,
    });
    builder.end_reactive().expect("end_reactive until");

    // 4. After rule (24 loops)
    builder.begin_after(24);
    builder.push_stmt(HirStmt::VarAssignConst {
        var: test_var,
        value: 99,
    });
    builder.end_reactive().expect("end_reactive after");

    // 5. Every rule (12 loops)
    builder.begin_every(12);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 3,
    });
    builder.end_reactive().expect("end_reactive every");

    let mut module = builder.finish().expect("finish HIR");
    let main_fn = module
        .functions()
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let stmts = &main_fn.blocks[0].stmts;

    // Check that HirModule contains 5 first-class HirStmt::Reactive before lowering
    let reactive_count = stmts
        .iter()
        .filter(|s| matches!(s, HirStmt::Reactive(_)))
        .count();
    assert_eq!(reactive_count, 5);

    // Run core lowering pass
    module.lower_reactive_rules().expect("lower reactive rules");

    let lowered_main_fn = module
        .functions()
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let lowered_stmts = &lowered_main_fn.blocks[0].stmts;

    // Verify zero HirStmt::Reactive left after lowering
    assert!(
        lowered_stmts
            .iter()
            .all(|s| !matches!(s, HirStmt::Reactive(_))),
        "all reactive rules must be lowered to standard HirStmt"
    );

    // Verify 4-byte variables allocated for guards and timers
    let var_names: Vec<&str> = module.variables().iter().map(|v| v.name.as_str()).collect();
    assert!(var_names.contains(&"once_guard"));
    assert!(var_names.contains(&"until_guard"));
    assert!(var_names.contains(&"after_timer"));
    assert!(var_names.contains(&"after_done"));
    assert!(var_names.contains(&"every_timer"));
}

#[test]
fn test_reactive_lowering_places_guards_conditions_and_bodies() {
    let mut builder = HirBuilder::new("test_reactive_shapes");
    let test_var = builder.add_var("test_var", 0);
    let cond = StockCondition::VarCmp {
        var: test_var,
        comparison: Comparison::AtLeast,
        value: 5,
        mask: None,
    };

    builder.begin_once(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 10,
    });
    builder.end_reactive().expect("end_reactive once");

    builder.begin_when(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 1,
    });
    builder.end_reactive().expect("end_reactive when");

    builder.begin_until(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 2,
    });
    builder.end_reactive().expect("end_reactive until");

    builder.begin_after(24);
    builder.push_stmt(HirStmt::VarAssignConst {
        var: test_var,
        value: 99,
    });
    builder.end_reactive().expect("end_reactive after");

    builder.begin_every(12);
    builder.push_stmt(HirStmt::VarAddConst {
        var: test_var,
        value: 3,
    });
    builder.end_reactive().expect("end_reactive every");

    let mut module = builder.finish().expect("finish HIR");
    assert_eq!(
        module.variables().len(),
        1,
        "lowering is the only place that allocates reactive guards and timers"
    );

    module.lower_reactive_rules().expect("lower reactive rules");

    let var_id = |module: &HirModule, name: &str| {
        module
            .variables()
            .iter()
            .find(|v| v.name == name)
            .unwrap_or_else(|| panic!("missing reactive variable {name}"))
            .id
    };
    let init_val = |module: &HirModule, id: VarId| module.variables()[id.0 as usize].init_val;
    let var_cmp = |var, comparison, value| StockCondition::VarCmp {
        var,
        comparison,
        value,
        mask: None,
    };

    let once_guard = var_id(&module, "once_guard");
    let until_guard = var_id(&module, "until_guard");
    let after_timer = var_id(&module, "after_timer");
    let after_done = var_id(&module, "after_done");
    let every_timer = var_id(&module, "every_timer");

    assert_eq!(init_val(&module, once_guard), 1);
    assert_eq!(init_val(&module, until_guard), 0);
    assert_eq!(init_val(&module, after_timer), 24);
    assert_eq!(init_val(&module, after_done), 0);
    assert_eq!(init_val(&module, every_timer), 12);

    let main_fn = module
        .functions()
        .iter()
        .find(|f| f.name == "main")
        .expect("main function");
    let stmts = &main_fn.blocks[0].stmts;
    assert_eq!(
        stmts.len(),
        6,
        "once/when/until/after emit one statement and every emits two"
    );

    // once: guard == 1 && condition, then clear the guard before the body.
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &stmts[0]
    else {
        panic!("once must lower to an If, got {:?}", stmts[0]);
    };
    assert!(else_body.is_empty());
    assert_eq!(conditions[0], var_cmp(once_guard, Comparison::Exactly, 1));
    assert_eq!(&conditions[1..], &[cond.clone()]);
    assert_eq!(
        then_body,
        &[
            HirStmt::VarAssignConst {
                var: once_guard,
                value: 0,
            },
            HirStmt::VarAddConst {
                var: test_var,
                value: 10,
            },
        ]
    );

    // when: the user condition is the only condition and the body is inline.
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &stmts[1]
    else {
        panic!("when must lower to an If, got {:?}", stmts[1]);
    };
    assert!(else_body.is_empty());
    assert_eq!(conditions, &vec![cond.clone()]);
    assert_eq!(
        then_body,
        &[HirStmt::VarAddConst {
            var: test_var,
            value: 1,
        }]
    );

    // until: guard == 0 wraps an If that either sets the guard or runs the body.
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &stmts[2]
    else {
        panic!("until must lower to an If, got {:?}", stmts[2]);
    };
    assert!(else_body.is_empty());
    assert_eq!(
        conditions,
        &vec![var_cmp(until_guard, Comparison::Exactly, 0)]
    );
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &then_body[0]
    else {
        panic!("until must nest the user condition, got {:?}", then_body[0]);
    };
    assert_eq!(conditions, &vec![cond.clone()]);
    assert_eq!(
        then_body,
        &[HirStmt::VarAssignConst {
            var: until_guard,
            value: 1,
        }]
    );
    assert_eq!(
        else_body,
        &[HirStmt::VarAddConst {
            var: test_var,
            value: 2,
        }]
    );

    // after: done == 0 wraps the countdown and the fire check.
    let HirStmt::If {
        conditions,
        then_body: after_body,
        else_body,
    } = &stmts[3]
    else {
        panic!("after must lower to an If, got {:?}", stmts[3]);
    };
    assert!(else_body.is_empty());
    assert_eq!(
        conditions,
        &vec![var_cmp(after_done, Comparison::Exactly, 0)]
    );
    assert_eq!(after_body.len(), 2);
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &after_body[0]
    else {
        panic!("after must count down the timer, got {:?}", after_body[0]);
    };
    assert!(else_body.is_empty());
    assert_eq!(
        conditions,
        &vec![var_cmp(after_timer, Comparison::AtLeast, 1)]
    );
    assert_eq!(
        then_body,
        &[HirStmt::VarSubConst {
            var: after_timer,
            value: 1,
        }]
    );
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &after_body[1]
    else {
        panic!("after must check the timer, got {:?}", after_body[1]);
    };
    assert!(else_body.is_empty());
    assert_eq!(
        conditions,
        &vec![var_cmp(after_timer, Comparison::Exactly, 0)]
    );
    assert_eq!(
        then_body,
        &[
            HirStmt::VarAssignConst {
                var: after_done,
                value: 1,
            },
            HirStmt::VarAssignConst {
                var: test_var,
                value: 99,
            },
        ]
    );

    // every: countdown then reload the timer and run the body.
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &stmts[4]
    else {
        panic!("every must count down the timer, got {:?}", stmts[4]);
    };
    assert!(else_body.is_empty());
    assert_eq!(
        conditions,
        &vec![var_cmp(every_timer, Comparison::AtLeast, 1)]
    );
    assert_eq!(
        then_body,
        &[HirStmt::VarSubConst {
            var: every_timer,
            value: 1,
        }]
    );
    let HirStmt::If {
        conditions,
        then_body,
        else_body,
    } = &stmts[5]
    else {
        panic!("every must check the timer, got {:?}", stmts[5]);
    };
    assert!(else_body.is_empty());
    assert_eq!(
        conditions,
        &vec![var_cmp(every_timer, Comparison::Exactly, 0)]
    );
    assert_eq!(
        then_body,
        &[
            HirStmt::VarAssignConst {
                var: every_timer,
                value: 12,
            },
            HirStmt::VarAddConst {
                var: test_var,
                value: 3,
            },
        ]
    );
}

#[test]
fn test_unclosed_reactive_rule_fails_finish() {
    let mut builder = HirBuilder::new("test_unclosed");
    builder.begin_after(48);
    let err = builder.finish().unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot finish HIR while a function or control-flow block is still open"),
        "expected unclosed error, got {err}"
    );
}
