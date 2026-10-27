use eudkit_core::hir::{Comparison, HirBuilder, StockAction, StockCondition};

#[test]
fn test_hir_builder_empty_creates_default_main() {
    let builder = HirBuilder::new("test_mod");
    let module = builder.finish().expect("finish must succeed");
    assert_eq!(module.name, "test_mod");
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "main");
    assert!(module.functions[0].is_main_loop);
}

#[test]
fn test_hir_builder_function_and_nested_control_flow() {
    let mut builder = HirBuilder::new("test_mod");
    let v_counter = builder.add_var("counter", 0);

    builder.begin_function("my_func", false, vec![v_counter], 0, Some(true));
    builder.push_var_add_const(v_counter, 1);

    // if (counter >= 10) { ... } elif (counter >= 5) { ... } else { ... }
    builder.begin_if(vec![StockCondition::VarCmp {
        var: v_counter,
        comparison: Comparison::AtLeast,
        value: 10,
        mask: None,
    }]);
    builder.push_action(StockAction::Victory);
    builder.end_if().expect("end_if ok");

    builder
        .begin_elif(vec![StockCondition::VarCmp {
            var: v_counter,
            comparison: Comparison::AtLeast,
            value: 5,
            mask: None,
        }])
        .expect("begin_elif ok");
    builder.push_var_sub_const(v_counter, 1);
    builder.end_elif().expect("end_elif ok");

    builder.begin_else().expect("begin_else ok");
    builder.push_var_assign_const(v_counter, 0);
    builder.end_else().expect("end_else ok");

    // while (counter >= 1) { counter -= 1; }
    builder.begin_while(vec![StockCondition::VarCmp {
        var: v_counter,
        comparison: Comparison::AtLeast,
        value: 1,
        mask: None,
    }]);
    builder.push_var_sub_const(v_counter, 1);
    builder.push_break();
    builder.end_while().expect("end_while ok");

    let fn_id = builder.end_function().expect("end_function ok");
    assert_eq!(fn_id.0, 0);

    let module = builder.finish().expect("finish ok");
    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.name, "my_func");
    assert!(!func.is_main_loop);
    assert_eq!(func.blocks[0].stmts.len(), 3); // add_const, if-elif-else, while
}

#[test]
fn test_hir_builder_unclosed_block_fails_finish() {
    let mut builder = HirBuilder::new("test_mod");
    builder.begin_if(vec![StockCondition::Always]);
    // Forgot to call end_if()

    let err = builder.finish().unwrap_err();
    assert!(
        err.to_string().contains("cannot finish HIR while a function or control-flow block is still open"),
        "expected unclosed error, got {err:?}"
    );
}

#[test]
fn test_hir_builder_elif_without_if_fails() {
    let mut builder = HirBuilder::new("test_mod");
    let err = builder
        .begin_elif(vec![StockCondition::Always])
        .unwrap_err();
    assert!(
        err.to_string().contains("must immediately follow an if"),
        "expected error, got {err:?}"
    );
}

#[test]
fn test_hir_builder_else_without_if_fails() {
    let mut builder = HirBuilder::new("test_mod");
    let err = builder.begin_else().unwrap_err();
    assert!(
        err.to_string().contains("must immediately follow an if"),
        "expected error, got {err:?}"
    );
}

#[test]
fn test_hir_builder_multiple_else_fails() {
    let mut builder = HirBuilder::new("test_mod");
    builder.begin_if(vec![StockCondition::Always]);
    builder.end_if().unwrap();

    builder.begin_else().unwrap();
    builder.end_else().unwrap();

    // Second else must fail
    let err = builder.begin_else().unwrap_err();
    assert!(
        err.to_string().contains("if block already has an else branch")
            || err.to_string().contains("else must immediately follow an if or elif block"),
        "expected error, got {err:?}"
    );
}

#[test]
fn test_hir_builder_root_stmts_attach_to_main() {
    let mut builder = HirBuilder::new("test_mod");
    let v = builder.add_var("v", 42);
    builder.push_var_add_const(v, 10);
    builder.push_action(StockAction::Defeat);

    let module = builder.finish().unwrap();
    assert_eq!(module.functions.len(), 1);
    let main_fn = &module.functions[0];
    assert_eq!(main_fn.name, "main");
    assert!(main_fn.is_main_loop);
    assert_eq!(main_fn.blocks[0].stmts.len(), 2);
}
