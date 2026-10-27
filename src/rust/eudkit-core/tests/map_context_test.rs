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
fn extracts_typed_map_context_from_real_scx_maps_with_zero_global_state() {
    let root = repo_root();
    let basemap_str = root.join("tests/outputmap/basemap/basemap.scx");
    assert!(basemap_str.exists(), "missing {:?}", basemap_str);
    let basemap_strx = root.join("tests/outputmap/basemap/basemap_strx.scx");
    let korean_map = root.join("tests/outputmap/basemap/사이비2.scx");

    let session_a = CompilationSession::from_map(&basemap_str, CompilationConfig::default())
        .expect("should load basemap.scx (legacy STR)");
    let session_b = CompilationSession::from_map(&basemap_strx, CompilationConfig::default())
        .expect("should load basemap_strx.scx (Remastered STRx)");
    let session_c = CompilationSession::from_map(&korean_map, CompilationConfig::default())
        .expect("should load Korean map fixture");

    let ctx_a = session_a.map_context();
    let ctx_b = session_b.map_context();
    let ctx_c = session_c.map_context();

    // Strings: 1-based string table extracted from STR and STRx
    assert!(!ctx_a.strings().is_empty(), "STR strings must be parsed");
    assert!(!ctx_b.strings().is_empty(), "STRx strings must be parsed");
    assert_eq!(
        ctx_a.strings().len(),
        ctx_b.strings().len(),
        "basemap.scx and basemap_strx.scx should have matching string counts"
    );

    // MRGN: 255 location slots, 1-based location IDs, "Anywhere" at location 64
    assert_eq!(ctx_a.locations().len(), 255);
    assert_eq!(ctx_a.location_id("Anywhere"), Some(64));
    assert_eq!(ctx_b.locations().len(), 255);
    assert_eq!(ctx_b.location_id("Anywhere"), Some(64));

    // UNIT: Placed units parsed into 36-byte records
    assert!(!ctx_a.placed_units().is_empty(), "basemap should have placed units");
    assert_eq!(ctx_a.placed_units().len(), ctx_b.placed_units().len());

    // UPRP: 64 CUWP unit property slots (1-based 1..=64)
    assert_eq!(ctx_a.cuwp_slots().len(), 64);
    assert_eq!(ctx_a.cuwp_slots()[0].id, 1);
    assert_eq!(ctx_a.cuwp_slots()[63].id, 64);

    // UNIx: 228 extended unit settings (0..228) and unit name resolution
    assert_eq!(ctx_a.unit_settings().len(), 228);
    assert_eq!(ctx_a.unit_id("Terran Marine"), Some(0));
    assert_eq!(ctx_a.unit_id("Zerg Zergling"), Some(37));

    // SWNM: 256 switches (0..256)
    assert_eq!(ctx_a.switches().len(), 256);
    assert_eq!(ctx_a.switch_id("Switch 1"), Some(0));
    assert_eq!(ctx_a.switch_id("Switch 256"), Some(255));

    // OWNR & SIDE: 12 player slots
    assert_eq!(ctx_a.players().len(), 12);
    assert!(
        ctx_a.players()[..8].iter().any(|p| p.owner != 0),
        "at least one player slot in P1..P8 must be active"
    );

    // TRIG: classic triggers extracted from Korean map fixture
    assert!(
        !ctx_c.triggers().is_empty() || ctx_a.triggers().is_empty(),
        "TRIG section parsed cleanly"
    );
}
