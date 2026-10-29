use eudkit_core::hir::{
    FormatPart, HirBasicBlock, HirExpr, HirFunction, HirModule, HirStmt, PrintTarget, StockAction,
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
fn format_template_parser_parses_mixed_specifiers_and_text() {
    let args = vec![
        HirExpr::Const(1),
        HirExpr::Const(0xABCD),
        HirExpr::Const(100),
    ];
    let template = "Player %d: score=0x%08X, hp=%d";
    let parts = eudkit_core::hir::parse_format_template(template, &args).unwrap();

    assert_eq!(
        parts,
        vec![
            FormatPart::Text("Player ".to_string()),
            FormatPart::Decimal(HirExpr::Const(1)),
            FormatPart::Text(": score=0x".to_string()),
            FormatPart::Hex(HirExpr::Const(0xABCD)),
            FormatPart::Text(", hp=".to_string()),
            FormatPart::Decimal(HirExpr::Const(100)),
        ]
    );
}

#[test]
fn format_print_expands_to_runtime_writes_and_compiles_scx() {
    let map_path = repo_root()
        .join("tests")
        .join("outputmap")
        .join("basemap")
        .join("basemap_strx.scx");

    let mut session =
        CompilationSession::from_map(&map_path, CompilationConfig::default()).unwrap();

    let str_buf = session
        .register_map_string("Initial static text buffer placeholder with plenty of length")
        .unwrap();

    let mut hir = HirModule::new("test_format_print");
    let score_var = hir.add_var("score", 12345);

    let mut main_fn = HirFunction::new_main_loop("main");
    let bb0 = main_fn.add_block(HirBasicBlock::new("entry"));

    // Format print into str_buf
    let parts = vec![
        FormatPart::Text("Score: ".to_string()),
        FormatPart::Decimal(HirExpr::Var(score_var)),
        FormatPart::Text(" (Hex: 0x".to_string()),
        FormatPart::Hex(HirExpr::Var(score_var)),
        FormatPart::Text(")\0".to_string()),
    ];

    main_fn
        .block_mut(bb0)
        .push_stmt(HirStmt::Action(StockAction::FormatPrint {
            target: PrintTarget::MapString(str_buf.id),
            parts,
            always_display: false,
        }));

    // Runtime formatting into a display string must keep the display action.
    main_fn
        .block_mut(bb0)
        .push_stmt(HirStmt::Action(StockAction::FormatPrint {
            target: PrintTarget::DisplayText,
            parts: vec![
                FormatPart::Text("Wave ".to_string()),
                FormatPart::Decimal(HirExpr::Var(score_var)),
            ],
            always_display: true,
        }));

    // Also test print_at line 0
    main_fn
        .block_mut(bb0)
        .push_stmt(HirStmt::Action(StockAction::PrintAt {
            line: 0,
            parts: vec![FormatPart::Text("Line 0 status: OK\0".to_string())],
        }));

    hir.add_function(main_fn);
    session.set_hir_module(hir);

    let lir = session.lower_to_lir().unwrap();
    let lir_dump = lir.dump_lir();

    // Runtime digits are written by patching a trigger action through the
    // variable processor, so their destinations appear as symbolic EPDs
    // rather than one fully constant action per character.
    assert!(
        lir_dump.contains("p1=epd(@"),
        "runtime formatting must lower to dynamic EPD writes:\n{lir_dump}"
    );

    // The DisplayText target still becomes a show-text action after expansion.
    assert!(
        lir_dump.contains("type=9 "),
        "formatted display text must survive expansion as DisplayText:\n{lir_dump}"
    );

    // The old CPString residue-stream path must no longer be reachable.
    assert!(
        !lir_dump.contains("FormatPrint") && !lir_dump.contains("PrintAt"),
        "formatted actions must be fully expanded before LIR emission"
    );

    // Compile into CHK bytes and verify compilation report
    let (chk_bytes, report) = session.compile_chk_bytes().unwrap();
    assert!(!chk_bytes.is_empty());
    assert!(report.trigger_count > 0);
    assert!(report.payload_size > 0);
}
