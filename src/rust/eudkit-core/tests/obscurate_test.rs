use eudkit_core::chk::Chk;
use eudkit_core::config::ProjectConfig;
use eudkit_core::hir::{
    Comparison, HirBasicBlock, HirByteBuf, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, Modifier, PlayerId, ResourceType, StockAction, StockCondition,
};
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
    let mut hir = HirModule::new("obscurate_test_module");
    let _buf_id = hir.add_byte_buf(HirByteBuf::new("test_buffer", vec![1, 2, 3, 4]));

    let mut main_fn = HirFunction::new_main_loop("main");
    let bb0 = main_fn.add_block(HirBasicBlock::new("entry"));
    let bb1 = main_fn.add_block(HirBasicBlock::new("branch1"));
    let bb2 = main_fn.add_block(HirBasicBlock::new("exit"));

    main_fn.block_mut(bb0).push_stmt(HirStmt::Trigger {
        label: Some("trig_1".to_string()),
        conditions: vec![StockCondition::Deaths {
            player: PlayerId::Player1,
            comparison: Comparison::AtLeast,
            amount: 5,
            unit: 0,
        }],
        actions: vec![
            StockAction::SetDeaths {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                value: HirValue::Const(10),
                unit: 0,
            },
            StockAction::DisplayText {
                text: "Protected Mission Objective: Destroy Enemy Command Center".to_string(),
                always_display: true,
            },
            StockAction::Comment {
                text: "DEBUG: This comment should be stripped when obscurate=true".to_string(),
            },
            StockAction::SetResources {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                amount: 50,
                resource: ResourceType::Ore,
            },
        ],
        preserved: true,
    });

    main_fn.block_mut(bb0).set_terminator(HirTerminator::Branch {
        conditions: vec![StockCondition::Deaths {
            player: PlayerId::Player2,
            comparison: Comparison::Exactly,
            amount: 0,
            unit: 0,
        }],
        then_block: bb1,
        else_block: bb2,
    });

    main_fn.block_mut(bb1).push_stmt(HirStmt::Action(StockAction::SetDeaths {
        player: PlayerId::Player2,
        modifier: Modifier::SetTo,
        value: HirValue::Const(1),
        unit: 0,
    }));
    main_fn.block_mut(bb1).set_terminator(HirTerminator::Jump(bb2));
    main_fn.block_mut(bb2).set_terminator(HirTerminator::Return);

    hir.add_function(main_fn);
    hir
}

#[test]
fn test_session_config_and_set_obscurate() {
    let root = repo_root();
    let map_path = root.join("tests/unittests/testmap.scx");
    let mut session =
        CompilationSession::from_map(&map_path, CompilationConfig::default()).unwrap();

    assert!(!session.config().obscurate);
    assert_eq!(session.config().obscurate_seed, None);

    session.set_obscurate(true);
    assert!(session.config().obscurate);

    session.set_obscurate_seed(0xDEAD_BEEF_CAFE);
    assert_eq!(session.config().obscurate_seed, Some(0xDEAD_BEEF_CAFE));
}

#[test]
fn test_project_config_parse_obscurate() {
    let toml = r#"
[project]
name = "protected_map"
input_map = "basemap.scx"
output_map = "release.scx"

[build]
obscurate = true
obscurate_seed = 0x1337_C0DE
"#;
    let cfg = ProjectConfig::parse_str(toml).unwrap();
    assert!(cfg.obscurate);
    assert_eq!(cfg.obscurate_seed, Some(0x1337_C0DE));

    let comp_cfg = cfg.to_compilation_config();
    assert!(comp_cfg.obscurate);
    assert_eq!(comp_cfg.obscurate_seed, Some(0x1337_C0DE));
}

#[test]
fn test_obscurate_false_default_is_deterministic_and_unobfuscated() {
    let root = repo_root();
    let input_map = root.join("tests/unittests/testmap.scx");
    let temp_dir = tempfile::tempdir().unwrap();
    let out_map_1 = temp_dir.path().join("plain1.scx");
    let out_map_2 = temp_dir.path().join("plain2.scx");

    let mut session1 =
        CompilationSession::from_map(&input_map, CompilationConfig::default()).unwrap();
    session1.set_hir_module(build_sample_hir());
    let _rep1 = session1.compile_map(&out_map_1).unwrap();

    let mut session2 =
        CompilationSession::from_map(&input_map, CompilationConfig::default()).unwrap();
    session2.set_hir_module(build_sample_hir());
    let _rep2 = session2.compile_map(&out_map_2).unwrap();

    let bytes1 = fs::read(&out_map_1).unwrap();
    let bytes2 = fs::read(&out_map_2).unwrap();
    assert_eq!(bytes1, bytes2, "obscurate=false must produce byte-for-byte identical output");

    let mut archive = Archive::open(&out_map_1, OpenArchiveFlags::empty()).unwrap();
    let chk_bytes = archive.open_file("staredit\\scenario.chk").unwrap().read_all().unwrap();
    let chk1 = Chk::parse(&chk_bytes).unwrap();
    let strx = chk1.section(b"STRx").unwrap();
    let needle = b"Protected Mission Objective";
    assert!(
        strx.windows(needle.len()).any(|w| w == needle),
        "obscurate=false must keep strings in plaintext"
    );
}

#[test]
fn test_obscurate_true_seeded_determinism_and_scx_validity() {
    let root = repo_root();
    let input_map = root.join("tests/unittests/testmap.scx");
    let temp_dir = tempfile::tempdir().unwrap();
    let out_map_1 = temp_dir.path().join("obs1.scx");
    let out_map_2 = temp_dir.path().join("obs2.scx");
    let out_plain = temp_dir.path().join("plain.scx");

    let cfg_seeded = CompilationConfig {
        obscurate: true,
        obscurate_seed: Some(123456789),
        preserve_classic_triggers: true,
    };

    let mut session1 = CompilationSession::from_map(&input_map, cfg_seeded.clone()).unwrap();
    session1.set_hir_module(build_sample_hir());
    session1.compile_map(&out_map_1).unwrap();

    let mut session2 = CompilationSession::from_map(&input_map, cfg_seeded).unwrap();
    session2.set_hir_module(build_sample_hir());
    session2.compile_map(&out_map_2).unwrap();

    let mut session_plain =
        CompilationSession::from_map(&input_map, CompilationConfig::default()).unwrap();
    session_plain.set_hir_module(build_sample_hir());
    session_plain.compile_map(&out_plain).unwrap();

    let bytes1 = fs::read(&out_map_1).unwrap();
    let bytes2 = fs::read(&out_map_2).unwrap();
    let plain_bytes = fs::read(&out_plain).unwrap();

    // 1. Seeded determinism: identical seed must produce byte-for-byte identical output
    assert_eq!(bytes1, bytes2, "seeded obscurate=true must be byte-for-byte reproducible");

    // 2. Obscuration must differ from plain build
    assert_ne!(bytes1, plain_bytes, "obscurate=true must produce obfuscated output differing from plain");

    // 3. StormLib and scenario.chk validity
    let mut archive = Archive::open(&out_map_1, OpenArchiveFlags::empty()).unwrap();
    let chk_bytes = archive.open_file("staredit\\scenario.chk").unwrap().read_all().unwrap();
    let chk = Chk::parse(&chk_bytes).unwrap();
    assert!(chk.section(b"VER ").is_some());
    assert!(chk.section(b"STRx").is_some());
    assert!(chk.section(b"TRIG").is_some());
    assert!(chk.section(b"MRGN").is_some());

    // 4. String protection: plaintext string literal should NOT appear unencrypted in STRx
    let strx = chk.section(b"STRx").unwrap();
    let needle = b"Protected Mission Objective";
    assert!(
        !strx.windows(needle.len()).any(|w| w == needle),
        "obscurate=true must scramble static string literals in STRx"
    );

    // 5. Debug comment stripped: comment text should not appear anywhere in TRIG or STRx
    let comment_needle = b"DEBUG: This comment should be stripped";
    let trig = chk.section(b"TRIG").unwrap();
    assert!(
        !trig.windows(comment_needle.len()).any(|w| w == comment_needle),
        "debug comments must be stripped from TRIG"
    );
    assert!(
        !strx.windows(comment_needle.len()).any(|w| w == comment_needle),
        "debug comments must be stripped from STRx"
    );
}

#[test]
fn test_obscurate_shuffles_object_offsets_and_randomizes_unit_offsets() {
    let root = repo_root();
    let input_map = root.join("tests/unittests/testmap.scx");

    let cfg_plain = CompilationConfig {
        obscurate: false,
        obscurate_seed: None,
        preserve_classic_triggers: true,
    };
    let mut session_plain = CompilationSession::from_map(&input_map, cfg_plain).unwrap();
    session_plain.set_hir_module(build_sample_hir());
    session_plain.lower_to_lir().unwrap();
    let plain_alloc = eudkit_core::allocator::allocate_lir_module(session_plain.lir_module().unwrap()).unwrap();

    let cfg_obs = CompilationConfig {
        obscurate: true,
        obscurate_seed: Some(987654321),
        preserve_classic_triggers: true,
    };
    let mut session_obs = CompilationSession::from_map(&input_map, cfg_obs).unwrap();
    session_obs.set_hir_module(build_sample_hir());
    session_obs.lower_to_lir().unwrap();

    // Randomize LIR
    let mut lir_obs = session_obs.lir_module().unwrap().clone();
    let mut rng = eudkit_core::obscurate::ObscureRng::new(987654321);
    eudkit_core::obscurate::apply_unit_offset_randomization(&mut lir_obs, &mut rng);

    // Verify unitid is randomized to non-zero for Deaths/SetDeaths
    let mut randomized_actions = 0;
    let mut randomized_conditions = 0;
    for obj in lir_obs.objects() {
        if let eudkit_core::lir::LirObject::Trigger(trig) = obj {
            for c in &trig.conditions {
                if c.condtype == 15 && c.unitid > 0 {
                    randomized_conditions += 1;
                }
            }
            for a in &trig.actions {
                if a.acttype == 45 && a.unitid > 0 {
                    randomized_actions += 1;
                }
            }
        }
    }
    assert!(randomized_conditions > 0, "conditions should have randomized unit offsets");
    assert!(randomized_actions > 0, "actions should have randomized unit offsets");

    // Permute allocation
    let obs_alloc = eudkit_core::allocator::allocate_lir_module_with_seed(&lir_obs, Some(987654321)).unwrap();
    assert_ne!(
        plain_alloc.object_offsets, obs_alloc.object_offsets,
        "object offsets must be permuted under obscuration"
    );
}

#[test]
fn test_obscurate_scrambles_mrgn_and_strips_swnm_and_inserts_restore_trigger() {
    let root = repo_root();
    let input_map = root.join("tests/unittests/testmap.scx");
    let temp_dir = tempfile::tempdir().unwrap();
    let out_plain = temp_dir.path().join("plain.scx");
    let out_obs = temp_dir.path().join("obs.scx");

    let mut session_plain =
        CompilationSession::from_map(&input_map, CompilationConfig::default()).unwrap();
    session_plain.set_hir_module(build_sample_hir());
    session_plain.compile_map(&out_plain).unwrap();

    let cfg_obs = CompilationConfig {
        obscurate: true,
        obscurate_seed: Some(55555),
        preserve_classic_triggers: true,
    };
    let mut session_obs = CompilationSession::from_map(&input_map, cfg_obs).unwrap();
    session_obs.set_hir_module(build_sample_hir());
    session_obs.compile_map(&out_obs).unwrap();

    // Check plain scenario.chk
    let mut arc_plain = Archive::open(&out_plain, OpenArchiveFlags::empty()).unwrap();
    let plain_chk_bytes = arc_plain.open_file("staredit\\scenario.chk").unwrap().read_all().unwrap();
    let plain_chk = Chk::parse(&plain_chk_bytes).unwrap();
    let plain_mrgn = plain_chk.section(b"MRGN").unwrap();

    // Check obscured scenario.chk
    let mut arc_obs = Archive::open(&out_obs, OpenArchiveFlags::empty()).unwrap();
    let obs_chk_bytes = arc_obs.open_file("staredit\\scenario.chk").unwrap().read_all().unwrap();
    let obs_chk = Chk::parse(&obs_chk_bytes).unwrap();
    let obs_mrgn = obs_chk.section(b"MRGN").unwrap();

    // 1. MRGN on disk must be scrambled
    assert_ne!(plain_mrgn, obs_mrgn, "MRGN section in CHK must be scrambled");

    // 2. SWNM must be stripped (zeros or empty)
    if let Some(swnm) = obs_chk.section(b"SWNM") {
        assert!(swnm.iter().all(|&b| b == 0), "SWNM switch names must be stripped");
    }

    // 3. TRIG must contain restore triggers that reference runtime MRGN (0x0058DC60, epd 3647)
    let trig_bytes = obs_chk.section(b"TRIG").unwrap();
    let trigger_count = trig_bytes.len() / 2400;
    assert!(trigger_count > 70, "TRIG should contain restore triggers in addition to bootstrap triggers");

    // Look for actions with player1 == epd(0x0058DC60) = 3647
    let mrgn_epd_target = 3647i32;
    let mut found_mrgn_restore = false;
    for t_idx in 0..trigger_count {
        let t_base = t_idx * 2400;
        for a_idx in 0..64 {
            let act_base = t_base + 320 + a_idx * 32;
            let player1 = i32::from_le_bytes(trig_bytes[act_base + 16..act_base + 20].try_into().unwrap());
            if player1 == mrgn_epd_target {
                found_mrgn_restore = true;
                break;
            }
        }
        if found_mrgn_restore {
            break;
        }
    }
    assert!(found_mrgn_restore, "TRIG must contain restore actions targeting runtime MRGN table at EPD 3647");
}

#[test]
fn test_obscurate_true_different_seeds_produce_different_outputs() {
    let root = repo_root();
    let input_map = root.join("tests/unittests/testmap.scx");
    let temp_dir = tempfile::tempdir().unwrap();
    let out_seed1 = temp_dir.path().join("seed1.scx");
    let out_seed2 = temp_dir.path().join("seed2.scx");

    let cfg1 = CompilationConfig {
        obscurate: true,
        obscurate_seed: Some(11111),
        preserve_classic_triggers: true,
    };
    let cfg2 = CompilationConfig {
        obscurate: true,
        obscurate_seed: Some(22222),
        preserve_classic_triggers: true,
    };

    let mut session1 = CompilationSession::from_map(&input_map, cfg1).unwrap();
    session1.set_hir_module(build_sample_hir());
    session1.compile_map(&out_seed1).unwrap();

    let mut session2 = CompilationSession::from_map(&input_map, cfg2).unwrap();
    session2.set_hir_module(build_sample_hir());
    session2.compile_map(&out_seed2).unwrap();

    let bytes1 = fs::read(&out_seed1).unwrap();
    let bytes2 = fs::read(&out_seed2).unwrap();
    assert_ne!(bytes1, bytes2, "different seeds must produce different permutations and encryptions");
}
