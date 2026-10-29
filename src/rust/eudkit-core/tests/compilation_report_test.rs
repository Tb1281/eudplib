use eudkit_core::hir::{
    HirBasicBlock, HirBinOp, HirExpr, HirFunction, HirModule, HirStmt, HirTerminator,
};
use eudkit_core::{CompilationConfig, CompilationReport, CompilationSession};
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

fn sample_report() -> CompilationReport {
    CompilationReport {
        trigger_count: 12,
        byte_buf_count: 3,
        vproc_count: 2,
        payload_offset: 4096,
        payload_size: 512,
        payload_entry_addr: 285_278_210,
        prt_offsets: vec![16, 32],
        ort_offsets: vec![48],
    }
}

#[test]
fn compilation_report_json_round_trips_every_field_including_vproc_count() {
    let report = sample_report();
    let json = report.to_json().expect("report should serialize");

    for key in [
        "trigger_count",
        "byte_buf_count",
        "vproc_count",
        "payload_offset",
        "payload_size",
        "payload_entry_addr",
        "prt_offsets",
        "ort_offsets",
    ] {
        assert!(
            json.contains(&format!("\"{key}\":")),
            "JSON report must contain required field {key}: {json}"
        );
    }

    assert_eq!(
        CompilationReport::from_json(&json).expect("report should deserialize"),
        report
    );
}

#[test]
fn compilation_report_json_rejects_missing_required_fields() {
    let json = sample_report().to_json().expect("report should serialize");

    let missing_vproc = json.replace("\"vproc_count\":2,", "");
    assert_ne!(missing_vproc, json);
    assert!(
        CompilationReport::from_json(&missing_vproc).is_err(),
        "vproc_count must be required instead of defaulting to zero"
    );

    let missing_payload = json.replace("\"payload_size\":512,", "");
    assert_ne!(missing_payload, json);
    assert!(CompilationReport::from_json(&missing_payload).is_err());

    assert!(CompilationReport::from_json("{\"trigger_count\":1}").is_err());
}

#[test]
fn compile_map_populates_vproc_count_from_the_lir_module() {
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_map = temp_dir.path().join("vproc_report.scx");

    let mut session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");

    let mut hir = HirModule::new("vproc_report_test");
    let a = hir.add_var("a", 12);
    let b = hir.add_var("b", 4);
    let out = hir.add_var("out", 0);

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry = main_fn.add_block(HirBasicBlock::new("entry"));
    main_fn.block_mut(entry).push_stmt(HirStmt::VarAssignExpr {
        var: out,
        expr: HirExpr::BinOp {
            op: HirBinOp::Mul,
            lhs: Box::new(HirExpr::Var(a)),
            rhs: Box::new(HirExpr::Var(b)),
        },
    });
    main_fn
        .block_mut(entry)
        .set_terminator(HirTerminator::Return);
    hir.add_function(main_fn);
    session.set_hir_module(hir);

    session
        .lower_to_lir()
        .expect("dynamic multiplication should lower to LIR");
    let expected_vprocs = session
        .lir_module()
        .expect("LIR module should exist")
        .vproc_count();
    assert!(
        expected_vprocs > 0,
        "dynamic multiplication should allocate VProc slots"
    );

    let report = session.compile_map(&output_map).expect("compile map");
    assert_eq!(
        report.vproc_count, expected_vprocs,
        "the Rust compile path must report the LIR VProc slot count"
    );
}
