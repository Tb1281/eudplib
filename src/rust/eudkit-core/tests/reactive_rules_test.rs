use eudkit_core::hir::{Comparison, HirBuilder, HirStmt, StockCondition};

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
fn test_reactive_lowering_guard_and_timer_shapes() {
    let mut builder = HirBuilder::new("test_reactive_shape");
    let value = builder.add_var("value", 0);
    let cond = StockCondition::VarCmp {
        var: value,
        comparison: Comparison::AtLeast,
        value: 5,
        mask: None,
    };

    builder.begin_once(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: value,
        value: 1,
    });
    builder.end_reactive().expect("end_reactive once");

    builder.begin_when(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: value,
        value: 2,
    });
    builder.end_reactive().expect("end_reactive when");

    builder.begin_until(vec![cond.clone()]);
    builder.push_stmt(HirStmt::VarAddConst {
        var: value,
        value: 3,
    });
    builder.end_reactive().expect("end_reactive until");

    builder.begin_after(24);
    builder.push_stmt(HirStmt::VarAddConst {
        var: value,
        value: 4,
    });
    builder.end_reactive().expect("end_reactive after");

    builder.begin_every(12);
    builder.push_stmt(HirStmt::VarAddConst {
        var: value,
        value: 5,
    });
    builder.end_reactive().expect("end_reactive every");

    let mut module = builder.finish().expect("finish HIR");
    module.lower_reactive_rules().expect("lower reactive rules");

    let once_guard = module
        .variables()
        .iter()
        .find(|v| v.name == "once_guard")
        .expect("once_guard allocated")
        .id;
    let until_guard = module
        .variables()
        .iter()
        .find(|v| v.name == "until_guard")
        .expect("until_guard allocated")
        .id;
    let after_timer = module
        .variables()
        .iter()
        .find(|v| v.name == "after_timer")
        .expect("after_timer allocated")
        .id;
    let after_done = module
        .variables()
        .iter()
        .find(|v| v.name == "after_done")
        .expect("after_done allocated")
        .id;
    let every_timer = module
        .variables()
        .iter()
        .find(|v| v.name == "every_timer")
        .expect("every_timer allocated")
        .id;

    let init_of = |name: &str| {
        module
            .variables()
            .iter()
            .find(|v| v.name == name)
            .expect("variable allocated")
            .init_val
    };
    assert_eq!(init_of("once_guard"), 1);
    assert_eq!(init_of("until_guard"), 0);
    assert_eq!(init_of("after_timer"), 24);
    assert_eq!(init_of("after_done"), 0);
    assert_eq!(init_of("every_timer"), 12);

    let main = module
        .functions()
        .iter()
        .find(|f| f.name == "main")
        .expect("main function");
    let stmts = &main.blocks[0].stmts;
    assert_eq!(
        stmts.len(),
        6,
        "once, when, until, after, and every(decr+fire)"
    );

    // Once: guard == 1 AND user condition, then clear the guard before the body.
    match &stmts[0] {
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            assert_eq!(
                conditions.as_slice(),
                &[
                    StockCondition::VarCmp {
                        var: once_guard,
                        comparison: Comparison::Exactly,
                        value: 1,
                        mask: None,
                    },
                    cond.clone(),
                ]
            );
            assert_eq!(
                then_body.as_slice(),
                &[
                    HirStmt::VarAssignConst {
                        var: once_guard,
                        value: 0,
                    },
                    HirStmt::VarAddConst {
                        var: value,
                        value: 1,
                    },
                ]
            );
            assert!(else_body.is_empty());
        }
        other => panic!("expected once guard If, got {other:?}"),
    }

    // When: the user condition is the only guard.
    match &stmts[1] {
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            assert_eq!(conditions.as_slice(), &[cond.clone()]);
            assert_eq!(
                then_body.as_slice(),
                &[HirStmt::VarAddConst {
                    var: value,
                    value: 2
                }]
            );
            assert!(else_body.is_empty());
        }
        other => panic!("expected when If, got {other:?}"),
    }

    // Until: guard == 0 wraps an If that sets the guard when the condition holds,
    // and runs the body in its else branch.
    match &stmts[2] {
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            assert_eq!(
                conditions.as_slice(),
                &[StockCondition::VarCmp {
                    var: until_guard,
                    comparison: Comparison::Exactly,
                    value: 0,
                    mask: None,
                }]
            );
            assert!(else_body.is_empty());
            match then_body.as_slice() {
                [HirStmt::If {
                    conditions: inner_conditions,
                    then_body: inner_then,
                    else_body: inner_else,
                }] => {
                    assert_eq!(inner_conditions.as_slice(), &[cond.clone()]);
                    assert_eq!(
                        inner_then.as_slice(),
                        &[HirStmt::VarAssignConst {
                            var: until_guard,
                            value: 1,
                        }]
                    );
                    assert_eq!(
                        inner_else.as_slice(),
                        &[HirStmt::VarAddConst {
                            var: value,
                            value: 3
                        }]
                    );
                }
                other => panic!("expected until inner If, got {other:?}"),
            }
        }
        other => panic!("expected until guard If, got {other:?}"),
    }

    // After: done == 0 wraps a timer decrement and a fire branch.
    match &stmts[3] {
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            assert_eq!(
                conditions.as_slice(),
                &[StockCondition::VarCmp {
                    var: after_done,
                    comparison: Comparison::Exactly,
                    value: 0,
                    mask: None,
                }]
            );
            assert!(else_body.is_empty());
            match then_body.as_slice() {
                [HirStmt::If {
                    conditions: decr_conditions,
                    then_body: decr_body,
                    else_body: decr_else,
                }, HirStmt::If {
                    conditions: fire_conditions,
                    then_body: fire_body,
                    else_body: fire_else,
                }] => {
                    assert_eq!(
                        decr_conditions.as_slice(),
                        &[StockCondition::VarCmp {
                            var: after_timer,
                            comparison: Comparison::AtLeast,
                            value: 1,
                            mask: None,
                        }]
                    );
                    assert_eq!(
                        decr_body.as_slice(),
                        &[HirStmt::VarSubConst {
                            var: after_timer,
                            value: 1,
                        }]
                    );
                    assert!(decr_else.is_empty());
                    assert_eq!(
                        fire_conditions.as_slice(),
                        &[StockCondition::VarCmp {
                            var: after_timer,
                            comparison: Comparison::Exactly,
                            value: 0,
                            mask: None,
                        }]
                    );
                    assert_eq!(
                        fire_body.as_slice(),
                        &[
                            HirStmt::VarAssignConst {
                                var: after_done,
                                value: 1,
                            },
                            HirStmt::VarAddConst {
                                var: value,
                                value: 4
                            },
                        ]
                    );
                    assert!(fire_else.is_empty());
                }
                other => panic!("expected after timer/fire Ifs, got {other:?}"),
            }
        }
        other => panic!("expected after done If, got {other:?}"),
    }

    // Every: a timer decrement followed by a fire branch that reloads the period.
    match &stmts[4] {
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            assert_eq!(
                conditions.as_slice(),
                &[StockCondition::VarCmp {
                    var: every_timer,
                    comparison: Comparison::AtLeast,
                    value: 1,
                    mask: None,
                }]
            );
            assert_eq!(
                then_body.as_slice(),
                &[HirStmt::VarSubConst {
                    var: every_timer,
                    value: 1,
                }]
            );
            assert!(else_body.is_empty());
        }
        other => panic!("expected every decrement If, got {other:?}"),
    }
    match &stmts[5] {
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            assert_eq!(
                conditions.as_slice(),
                &[StockCondition::VarCmp {
                    var: every_timer,
                    comparison: Comparison::Exactly,
                    value: 0,
                    mask: None,
                }]
            );
            assert_eq!(
                then_body.as_slice(),
                &[
                    HirStmt::VarAssignConst {
                        var: every_timer,
                        value: 12,
                    },
                    HirStmt::VarAddConst {
                        var: value,
                        value: 5
                    },
                ]
            );
            assert!(else_body.is_empty());
        }
        other => panic!("expected every fire If, got {other:?}"),
    }
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
