use eudkit_core::hir::{HirStmt, Modifier, PlayerId, StockAction};
use eudkit_core::{CompilationConfig, CompilationSession};
use eudkit_ts::compile_ts_to_hir;
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
fn compiles_typescript_functions_control_flow_and_actions_to_hir() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();

    let ts_source = r#"
import { var_, Player1, Add, SetDeaths, SetResources, Ore, Gas } from "@eudkit/sdk";

let a = var_(0, "a");
let b = var_(20, "b");
let c = var_(0, "c");

export function main() {
    // 1. Metaprogramming compile-time loop unrolling
    for (let i = 0; i < 40; i++) {
        a += 1;
        SetDeaths(Player1, Add, 1, "Terran Marine");
    }

    // 2. Structured if / else if / else
    if (a >= 30) {
        c = 100;
        SetResources(Player1, Add, 500, Ore);
    } else if (a == 15) {
        c = 50;
    } else {
        c = 10;
    }

    // 3. Structured while with break and continue
    while (b >= 1) {
        b -= 1;
        if (b == 5) {
            a += 10;
            continue;
        }
        if (a >= 200) {
            break;
        }
        a += 2;
    }

    // 4. Switch statement lowering
    switch (c) {
        case 100:
            SetResources(Player1, Add, 10, Gas);
            break;
        default:
            break;
    }
}
"#;

    let hir = compile_ts_to_hir(ts_source, "ts_test_module", map_ctx)
        .expect("should compile TS to HIR");

    assert_eq!(hir.name, "ts_test_module");
    assert_eq!(hir.variables().len(), 3);
    assert_eq!(hir.variables()[0].name, "a");
    assert_eq!(hir.variables()[1].name, "b");
    assert_eq!(hir.variables()[2].name, "c");

    assert_eq!(hir.functions().len(), 1);
    let main_fn = &hir.functions()[0];
    assert_eq!(main_fn.name, "main");
    assert!(main_fn.is_main_loop);

    let stmts = &main_fn.blocks[0].stmts;
    // 40 unrolled iterations: each has VarAddConst(a, 1) and Action(SetDeaths) -> 80 statements
    assert!(
        stmts.len() >= 80,
        "expected at least 80 statements from loop unrolling, got {}",
        stmts.len()
    );

    // Verify first unrolled iteration
    match &stmts[0] {
        HirStmt::VarAddConst { var, value } => {
            assert_eq!(var.0, 0);
            assert_eq!(value, &1);
        }
        other => panic!("expected VarAddConst at stmts[0], got {:?}", other),
    }

    match &stmts[1] {
        HirStmt::Action(StockAction::SetDeaths { player, modifier, unit, .. }) => {
            assert_eq!(player, &PlayerId::Player1);
            assert_eq!(modifier, &Modifier::Add);
            assert_eq!(unit, &0); // "Terran Marine" ID is 0
        }
        other => panic!("expected SetDeaths at stmts[1], got {:?}", other),
    }

    // Check that stmts contain If and While statements
    let has_if = stmts.iter().any(|s| matches!(s, HirStmt::If { .. }));
    let has_while = stmts.iter().any(|s| matches!(s, HirStmt::While { .. }));
    assert!(has_if, "HIR must contain lowered If statements");
    assert!(has_while, "HIR must contain lowered While statement");
}

#[test]
fn compiles_typescript_structs_helper_functions_and_arithmetic_ops() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();

    let ts_source = r#"
import { var_, CUnit, EPD, SetDeaths, Player1, Add } from "@eudkit/sdk";

let hp = var_(100, "hp");
let unit_epd = var_(EPD(0x59CCA8), "unit_epd");

function clampHp(val: number, max: number): number {
    if (val > max) {
        return max;
    }
    return val;
}

export function main() {
    let u = CUnit.cast(unit_epd);
    u.hitPoints = 1000;
    u.shields += 50;

    let next_hp = (hp + 20) * 2;
    clampHp(next_hp, 500);
}
"#;

    let hir = compile_ts_to_hir(ts_source, "struct_test", map_ctx)
        .expect("should compile TS struct test");

    // 2 global vars (hp, unit_epd) + 2 params (clampHp_val, clampHp_max) + 1 local var (next_hp) = 5 vars
    assert_eq!(hir.variables().len(), 5);
    assert_eq!(hir.variables()[0].name, "hp");
    assert_eq!(hir.variables()[1].name, "unit_epd");
    assert_eq!(hir.functions().len(), 2);

    let clamp_fn = hir.functions().iter().find(|f| f.name == "clampHp").unwrap();
    assert!(!clamp_fn.is_main_loop);

    let main_fn = hir.functions().iter().find(|f| f.name == "main").unwrap();
    assert!(main_fn.is_main_loop);

    let stmts = &main_fn.blocks[0].stmts;
    // Verify CUnit hitPoints SetDeathsDynamic
    let has_hp_set = stmts.iter().any(|s| match s {
        HirStmt::Action(StockAction::SetDeathsDynamic { modifier, unit, .. }) => {
            *modifier == Modifier::SetTo && *unit == 0
        }
        _ => false,
    });
    assert!(has_hp_set, "must have SetDeathsDynamic for CUnit hitPoints = 1000");

    // Verify helper function call
    let has_call = stmts.iter().any(|s| matches!(s, HirStmt::Call { .. }));
    assert!(has_call, "must have HirStmt::Call to clampHp");
}
