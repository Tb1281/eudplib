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

    let hir =
        compile_ts_to_hir(ts_source, "ts_test_module", map_ctx).expect("should compile TS to HIR");

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
        HirStmt::Action(StockAction::SetDeaths {
            player,
            modifier,
            unit,
            ..
        }) => {
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

    let clamp_fn = hir
        .functions()
        .iter()
        .find(|f| f.name == "clampHp")
        .unwrap();
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
    assert!(
        has_hp_set,
        "must have SetDeathsDynamic for CUnit hitPoints = 1000"
    );

    // Verify helper function call
    let has_call = stmts.iter().any(|s| matches!(s, HirStmt::Call { .. }));
    assert!(has_call, "must have HirStmt::Call to clampHp");
}

#[test]
fn compiles_typescript_format_print_and_user_cp() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();

    let ts_source = r#"
import { var_, IsUserCP, f_eprintln, print_at, display_at, displayAt } from "@eudkit/sdk";

let score = var_(150, "score");

export function main() {
    if (IsUserCP()) {
        f_eprintln("Current score: %d pts", score);
        print_at(3, "Fixed Line: %d", score);
        display_at(4, "DisplayAt Line: %d", score);
        displayAt(5, "CamelCase Line: %d", score);
    }
}
"#;

    let hir = compile_ts_to_hir(ts_source, "ts_print_test", map_ctx)
        .expect("should compile TS print test");

    assert_eq!(hir.variables().len(), 1);
    assert_eq!(hir.variables()[0].name, "score");

    let main_fn = hir.functions().iter().find(|f| f.name == "main").unwrap();
    let stmts = &main_fn.blocks[0].stmts;

    let has_if_user_cp = stmts.iter().any(|s| match s {
        HirStmt::If {
            conditions,
            then_body,
            ..
        } => {
            conditions
                .iter()
                .any(|c| matches!(c, eudkit_core::hir::StockCondition::IsUserCp))
                && then_body
                    .iter()
                    .any(|bs| matches!(bs, HirStmt::Action(StockAction::FormatPrint { .. })))
                && then_body
                    .iter()
                    .any(|bs| matches!(bs, HirStmt::Action(StockAction::PrintAt { .. })))
        }
        _ => false,
    });
    assert!(
        has_if_user_cp,
        "must compile If(IsUserCP()) with FormatPrint and PrintAt actions"
    );
}

#[test]
fn compiles_typescript_reactive_primitives_once_when_until_after_every() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();

    let ts_source = r#"
import { var_, once, when, until, after, every, DisplayText, SetResources, Player1, Add, Ore } from "@eudkit/sdk";

let kills = var_(0, "kills");
let timer = var_(10, "timer");
let score = var_(50, "score");

export function main() {
    once(() => {
        DisplayText("Game Initialized once");
        SetResources(Player1, Add, 50, Ore);
    });

    once(score >= 100, () => {
        DisplayText("Score milestone reached!");
    });

    when(kills >= 10, () => {
        DisplayText("10 kills achieved");
    });

    until(timer == 0, () => {
        DisplayText("Timer still running");
    });

    after(24, () => {
        DisplayText("24 trigger loops elapsed");
    });

    every(12, () => {
        DisplayText("Periodic 12 loops tick");
    });
}
"#;

    let mut hir = compile_ts_to_hir(ts_source, "ts_reactive_test", map_ctx)
        .expect("should compile TS reactive primitives to HIR");

    let main_fn = hir.functions().iter().find(|f| f.name == "main").unwrap();
    let stmts = &main_fn.blocks[0].stmts;

    // Verify presence of first-class reactive statements in HIR
    let reactive_count = stmts
        .iter()
        .filter(|s| matches!(s, HirStmt::Reactive(_)))
        .count();
    assert_eq!(
        reactive_count, 6,
        "must emit 6 first-class reactive statements into HIR, got {reactive_count}"
    );

    // Lower reactive rules into 4-byte variables and trigger conditions
    hir.lower_reactive_rules()
        .expect("should lower reactive rules");

    // Check user vars + reactive guard/timer vars
    let var_names: Vec<&str> = hir.variables().iter().map(|v| v.name.as_str()).collect();
    assert!(var_names.contains(&"kills"));
    assert!(var_names.contains(&"timer"));
    assert!(var_names.contains(&"score"));
    assert!(var_names.contains(&"once_guard"));
    assert!(var_names.contains(&"until_guard"));
    assert!(var_names.contains(&"after_timer"));
    assert!(var_names.contains(&"after_done"));
    assert!(var_names.contains(&"every_timer"));

    // Test compiling into full session
    let mut comp_session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    comp_session.set_hir_module(hir);
    let tmp = tempfile::tempdir().unwrap();
    let out_map = tmp.path().join("ts_reactive_out.scx");
    let report = comp_session
        .compile_map(&out_map)
        .expect("should compile to .scx");
    assert!(out_map.exists());
    assert!(report.trigger_count > 0);
    assert!(report.payload_size > 0);
}

#[test]
fn compiles_typescript_with_ambient_session_and_string_buffer() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();

    let ts_source = r#"
import { session, DisplayText, display_at, IsUserCP } from "eudkit";

let kills = session.var(42, "kills");
let hud = session.string("BOSS HP: %d / 1000");

export function main() {
    if (IsUserCP()) {
        DisplayText("Welcome to the Boss Arena!");
        hud.display_at(0, kills);
        display_at(1, "Fixed Line: %d", kills);
    }
}
"#;

    let hir = compile_ts_to_hir(ts_source, "ts_ambient_session_test", map_ctx)
        .expect("should compile TS ambient session test");

    assert_eq!(hir.variables().len(), 1);
    assert_eq!(hir.variables()[0].name, "kills");
    assert_eq!(hir.variables()[0].init_val, 42);

    let main_fn = hir.functions().iter().find(|f| f.name == "main").unwrap();
    let stmts = &main_fn.blocks[0].stmts;

    let has_if_user_cp = stmts.iter().any(|s| match s {
        HirStmt::If {
            conditions,
            then_body,
            ..
        } => {
            conditions
                .iter()
                .any(|c| matches!(c, eudkit_core::hir::StockCondition::IsUserCp))
                && then_body
                    .iter()
                    .any(|bs| matches!(bs, HirStmt::Action(StockAction::DisplayText { .. })))
                && then_body
                    .iter()
                    .any(|bs| matches!(bs, HirStmt::Action(StockAction::PrintAt { line: 0, .. })))
                && then_body
                    .iter()
                    .any(|bs| matches!(bs, HirStmt::Action(StockAction::PrintAt { line: 1, .. })))
        }
        _ => false,
    });
    assert!(
        has_if_user_cp,
        "must compile If(IsUserCP()) with DisplayText and PrintAt actions"
    );
}

#[test]
fn compiles_typescript_memory_reads_consistently_for_declarations_and_assignments() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();

    let ts_source = r#"
import {
    dwread_epd,
    dwread,
    epdread_epd,
    bread_epd,
    bread,
    wread_epd,
    wread,
} from "@eudkit/sdk";

export function main() {
    let v0 = dwread_epd(100);
    let v1 = dwread(100);
    let v2 = epdread_epd(100);
    let v3 = bread_epd(100);
    let v4 = bread(100);
    let v5 = wread_epd(100);
    let v6 = wread(100);

    v0 = eudkit.dwread_epd(100);
    v1 = eudkit.dwread(100);
    v2 = eudkit.epdread_epd(100);
    v3 = eudkit.bread_epd(100);
    v4 = eudkit.bread(100);
    v5 = eudkit.wread_epd(100);
    v6 = eudkit.wread(100);
}
"#;

    let hir = compile_ts_to_hir(ts_source, "ts_mem_read_test", map_ctx)
        .expect("should compile TS memory reads");
    let main_fn = hir.functions().iter().find(|f| f.name == "main").unwrap();
    let reads: Vec<&HirStmt> = main_fn.blocks[0]
        .stmts
        .iter()
        .filter(|stmt| matches!(stmt, HirStmt::MemRead { .. }))
        .collect();

    assert_eq!(
        reads.len(),
        14,
        "seven declaration reads and seven assignment reads"
    );

    let expected = [
        (0xFFFF_FFFFu32, 0i32),
        (0xFFFF_FFFF, 0),
        (0xFFFF_FFFF, -(0x0058_A364 / 4)),
        (0xFF, 0),
        (0xFF, 0),
        (0xFFFF, 0),
        (0xFFFF, 0),
    ];

    for (index, (mask, initial_val)) in expected.iter().enumerate() {
        let (
            HirStmt::MemRead {
                epd: decl_epd,
                mask: decl_mask,
                shift: decl_shift,
                initial_val: decl_init,
                ..
            },
            HirStmt::MemRead {
                epd: assign_epd,
                mask: assign_mask,
                shift: assign_shift,
                initial_val: assign_init,
                ..
            },
        ) = (reads[index], reads[index + 7])
        else {
            panic!("expected MemRead statements for memory-read rule {index}");
        };

        assert_eq!(
            decl_epd, assign_epd,
            "declaration and assignment must lower the same EPD expression for rule {index}"
        );
        assert_eq!(decl_mask, assign_mask);
        assert_eq!(decl_shift, assign_shift);
        assert_eq!(decl_init, assign_init);
        assert_eq!(*decl_mask, *mask);
        assert_eq!(*decl_init, *initial_val);
    }
}
