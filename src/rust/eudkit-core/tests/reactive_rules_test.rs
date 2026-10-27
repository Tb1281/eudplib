use eudkit_core::hir::{
    Comparison, HirBuilder, HirStmt, ReactiveRule, StockCondition,
};

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
    let main_fn = module.functions().iter().find(|f| f.name == "main").unwrap();
    let stmts = &main_fn.blocks[0].stmts;

    // Check that HirModule contains 5 first-class HirStmt::Reactive before lowering
    let reactive_count = stmts
        .iter()
        .filter(|s| matches!(s, HirStmt::Reactive(_)))
        .count();
    assert_eq!(reactive_count, 5);

    // Run core lowering pass
    module.lower_reactive_rules().expect("lower reactive rules");

    let lowered_main_fn = module.functions().iter().find(|f| f.name == "main").unwrap();
    let lowered_stmts = &lowered_main_fn.blocks[0].stmts;

    // Verify zero HirStmt::Reactive left after lowering
    assert!(
        lowered_stmts.iter().all(|s| !matches!(s, HirStmt::Reactive(_))),
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
fn test_unclosed_reactive_rule_fails_finish() {
    let mut builder = HirBuilder::new("test_unclosed");
    builder.begin_after(48);
    let err = builder.finish().unwrap_err();
    assert!(
        err.to_string().contains("cannot finish HIR while a function or control-flow block is still open"),
        "expected unclosed error, got {err}"
    );
}
