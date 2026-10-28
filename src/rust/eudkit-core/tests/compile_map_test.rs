use eudkit_core::chk::Chk;
use eudkit_core::hir::{
    epd, Comparison, HirBasicBlock, HirByteBuf, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, Modifier, PlayerId, ResourceType, StockAction, StockCondition, STRX_BASE_ADDR,
};
use eudkit_core::lir::{ActField, FieldRef};
use eudkit_core::{CompilationConfig, CompilationSession};
use eudplib_stormlib::{Archive, OpenArchiveFlags};
use std::fs;
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

fn build_sample_hir() -> HirModule {
    let mut hir = HirModule::new("tracer_e2e");
    let buf_id = hir.add_byte_buf(HirByteBuf::new("shared_cell", vec![0x78, 0x56, 0x34, 0x12]));

    let mut main_fn = HirFunction::new_main_loop("main");
    let bb0 = main_fn.add_block(HirBasicBlock::new("entry"));
    let bb1 = main_fn.add_block(HirBasicBlock::new("then_branch"));
    let bb2 = main_fn.add_block(HirBasicBlock::new("done"));

    let act_val_ref = FieldRef::action(0, 0, ActField::Player2);

    main_fn.block_mut(bb0).push_stmt(HirStmt::Trigger {
        label: Some("tick_body".to_string()),
        conditions: vec![StockCondition::Deaths {
            player: PlayerId::Player1,
            comparison: Comparison::AtLeast,
            amount: 0,
            unit: 0,
        }],
        actions: vec![
            StockAction::SetDeaths {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                value: HirValue::Const(1),
                unit: 0,
            },
            StockAction::SetField {
                target: act_val_ref,
                modifier: Modifier::SetTo,
                value: HirValue::ByteBufAddress(buf_id, 0),
            },
            StockAction::SetResources {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                amount: 100,
                resource: ResourceType::Ore,
            },
            StockAction::DisplayText {
                text: "eudkit deterministic build test".to_string(),
                always_display: true,
            },
        ],
        preserved: true,
    });
    main_fn
        .block_mut(bb0)
        .set_terminator(HirTerminator::Branch {
            conditions: vec![StockCondition::Deaths {
                player: PlayerId::Player1,
                comparison: Comparison::AtLeast,
                amount: 10,
                unit: 0,
            }],
            then_block: bb1,
            else_block: bb2,
        });

    main_fn
        .block_mut(bb1)
        .push_stmt(HirStmt::Action(StockAction::SetResources {
            player: PlayerId::Player1,
            modifier: Modifier::Add,
            amount: 500,
            resource: ResourceType::Gas,
        }));
    main_fn
        .block_mut(bb1)
        .set_terminator(HirTerminator::Jump(bb2));
    main_fn.block_mut(bb2).set_terminator(HirTerminator::Return);

    hir.add_function(main_fn);
    hir
}

#[test]
fn compiles_scx_map_with_object_stacking_static_strx_relocation_and_byte_determinism() {
    let root = repo_root();
    let input_map = root.join("tests/unittests/testmap.scx");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let out_map_1 = temp_dir.path().join("out1.scx");
    let out_map_2 = temp_dir.path().join("out2.scx");

    // Compile Run #1
    let mut session1 =
        CompilationSession::from_map(&input_map, CompilationConfig::default()).unwrap();
    session1.set_hir_module(build_sample_hir());
    let report1 = session1.compile_map(&out_map_1).expect("compile_map run 1");

    // Compile Run #2
    let mut session2 =
        CompilationSession::from_map(&input_map, CompilationConfig::default()).unwrap();
    session2.set_hir_module(build_sample_hir());
    let report2 = session2.compile_map(&out_map_2).expect("compile_map run 2");

    // 1. Assert 100% byte-for-byte determinism across repeated runs
    let bytes1 = fs::read(&out_map_1).unwrap();
    let bytes2 = fs::read(&out_map_2).unwrap();
    assert_eq!(
        bytes1, bytes2,
        "output .scx archive must be 100% byte-for-byte deterministic across runs"
    );
    assert_eq!(report1, report2);

    // 2. Assert object-stacking packed the 4-byte ByteBuf inside unused trigger space
    assert!(
        report1.trigger_count >= 4,
        "should emit triggers for entry, branch reset/cond, then_branch, and done"
    );
    assert!(
        report1.payload_size < report1.trigger_count * 2408,
        "sparse triggers and 4-byte ByteBuf must stack into fewer bytes than unstacked triggers ({} < {})",
        report1.payload_size,
        report1.trigger_count * 2408
    );
    assert_eq!(
        report1.payload_offset % 4,
        0,
        "payload offset in STRx must be 4-byte aligned"
    );

    // 3. Open output .scx in StormLib and verify scenario.chk integrity
    let mut archive = Archive::open(&out_map_1, OpenArchiveFlags::empty())
        .expect("output .scx must open cleanly in StormLib");
    let raw_chk = archive
        .open_file("staredit\\scenario.chk")
        .expect("output .scx must contain staredit\\scenario.chk")
        .read_all()
        .expect("should read scenario.chk");

    // Standard non-obscurated builds must NOT emit fake negative ISOM footers so map editors can open the map
    let out_chk = Chk::parse(&raw_chk).expect("output scenario.chk must parse cleanly");
    assert!(
        !out_chk.has_isom_footer(),
        "standard builds must not append fake negative ISOM footers"
    );
    assert!(
        out_chk.section(b"STR ").is_some(),
        "classic STR section must be preserved alongside STRx for map editor compatibility"
    );

    // 4. Verify STRx section & static 0x191943C8 relocations
    let strx = out_chk.section(b"STRx").expect("STRx section must exist");
    assert_eq!(
        strx.len(),
        report1.payload_offset + report1.payload_size,
        "STRx must end with the 4-byte aligned relocated payload"
    );

    let payload_base_addr = STRX_BASE_ADDR + report1.payload_offset as u32;
    let payload_end_addr = payload_base_addr + report1.payload_size as u32;
    let payload_slice = &strx[report1.payload_offset..];

    assert!(
        !report1.ort_offsets.is_empty(),
        "should have byte-pointer relocations"
    );
    assert!(
        !report1.prt_offsets.is_empty(),
        "should have EPD relocations"
    );

    for &ort in &report1.ort_offsets {
        assert_eq!(ort % 4, 0, "orttable offset must be 4-byte aligned");
        let relocated_ptr = u32::from_le_bytes(payload_slice[ort..ort + 4].try_into().unwrap());
        assert!(
            relocated_ptr >= payload_base_addr && relocated_ptr < payload_end_addr,
            "relocated byte pointer {relocated_ptr:#010X} must lie within [{payload_base_addr:#010X}, {payload_end_addr:#010X})"
        );
        assert_eq!(
            relocated_ptr % 4,
            0,
            "relocated object/field pointer must be 4-byte aligned"
        );
    }

    let min_epd = epd(payload_base_addr);
    let max_epd = epd(payload_end_addr);
    for &prt in &report1.prt_offsets {
        assert_eq!(prt % 4, 0, "prttable offset must be 4-byte aligned");
        let relocated_epd = i32::from_le_bytes(payload_slice[prt..prt + 4].try_into().unwrap());
        assert!(
            relocated_epd >= min_epd && relocated_epd < max_epd,
            "relocated EPD {relocated_epd} must lie within [{min_epd}, {max_epd})"
        );
    }

    // 5. Verify TRIG bootstrap section
    let trig = out_chk.section(b"TRIG").expect("TRIG section must exist");
    assert_eq!(trig.len() % 2400, 0);
    assert_eq!(
        trig.len() / 2400,
        session1.map_context().triggers().len() + 70,
        "TRIG must contain preserved classic triggers plus 70 single-stage bootstrap triggers"
    );
}

#[test]
fn preserves_classic_trig_triggers_and_upgrades_legacy_str_to_strx() {
    let root = repo_root();
    let legacy_str_map = root.join("tests/outputmap/basemap/basemap.scx");
    let korean_trig_map = root.join("tests/outputmap/basemap/사이비2.scx");
    let temp_dir = tempfile::tempdir().expect("tempdir");

    // 1. Compile legacy STR map (basemap.scx) -> verify STR is upgraded to STRx
    let out_legacy = temp_dir.path().join("legacy_upgraded.scx");
    let mut session_legacy =
        CompilationSession::from_map(&legacy_str_map, CompilationConfig::default()).unwrap();
    assert!(
        session_legacy.chk().section(b"STR ").is_some(),
        "basemap.scx starts with legacy STR section"
    );
    session_legacy.set_hir_module(build_sample_hir());
    let report_legacy = session_legacy.compile_map(&out_legacy).unwrap();

    let mut arch_legacy = Archive::open(&out_legacy, OpenArchiveFlags::empty()).unwrap();
    let chk_legacy_bytes = arch_legacy
        .open_file("staredit\\scenario.chk")
        .unwrap()
        .read_all()
        .unwrap();
    let chk_legacy = Chk::parse(&chk_legacy_bytes).unwrap();
    assert!(
        chk_legacy.section(b"STR ").is_some(),
        "STR section must be emitted alongside STRx for map editor compatibility"
    );
    let strx_legacy = chk_legacy.section(b"STRx").unwrap();
    assert_eq!(
        strx_legacy.len(),
        report_legacy.payload_offset + report_legacy.payload_size
    );

    // 2. Compile map with classic TRIG triggers -> verify classic triggers are preserved intact
    let out_korean = temp_dir.path().join("korean_out.scx");
    let mut session_kr =
        CompilationSession::from_map(&korean_trig_map, CompilationConfig::default()).unwrap();
    let orig_triggers = session_kr.map_context().triggers().to_vec();
    assert!(
        !orig_triggers.is_empty(),
        "Korean map fixture should have classic TRIG triggers"
    );
    session_kr.set_hir_module(build_sample_hir());
    session_kr.compile_map(&out_korean).unwrap();

    let mut arch_kr = Archive::open(&out_korean, OpenArchiveFlags::empty()).unwrap();
    let chk_kr_bytes = arch_kr
        .open_file("staredit\\scenario.chk")
        .unwrap()
        .read_all()
        .unwrap();
    let chk_kr = Chk::parse(&chk_kr_bytes).unwrap();
    let trig_kr = chk_kr.section(b"TRIG").unwrap();
    assert_eq!(trig_kr.len() / 2400, orig_triggers.len() + 70);
    for (i, orig) in orig_triggers.iter().enumerate() {
        assert_eq!(
            &trig_kr[i * 2400..(i + 1) * 2400],
            orig.raw.as_slice(),
            "classic trigger #{i} must be preserved intact"
        );
    }
}
