use eudkit_core::hir::{epd, STRX_BASE_ADDR};
use eudkit_core::{CompilationConfig, CompilationSession, MapString};
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
fn map_string_registers_deduplicates_and_resolves_addresses_in_session() {
    let map_path = repo_root()
        .join("tests")
        .join("outputmap")
        .join("basemap")
        .join("basemap_strx.scx");

    let mut session =
        CompilationSession::from_map(&map_path, CompilationConfig::default()).unwrap();

    let s1: MapString = session.register_map_string("Hello World").unwrap();
    let s2 = session.register_map_string("Hello World").unwrap();
    let s3 = session.register_map_string("Different String").unwrap();

    // Deduplication check: s1 and s2 must share the same string ID and address
    assert_eq!(s1.id, s2.id);
    assert_eq!(s1.text, s2.text);
    assert_eq!(s1.offset, s2.offset);
    assert_eq!(s1.address, s2.address);
    assert_eq!(s1.epd, s2.epd);

    // Differentiation check: s3 must have a distinct ID and address
    assert_ne!(s1.id, s3.id);
    assert_ne!(s1.address, s3.address);

    // Address calculation invariant: address = STRX_BASE_ADDR + offset, epd = epd(address)
    assert_eq!(s1.address, STRX_BASE_ADDR + s1.offset);
    assert_eq!(s1.epd, epd(s1.address));
    assert_eq!(s3.address, STRX_BASE_ADDR + s3.offset);
    assert_eq!(s3.epd, epd(s3.address));

    // Verify StringTable can read bytes back
    assert_eq!(
        session.map_data().string_table().get_by_id(s1.id),
        Some(b"Hello World".as_slice())
    );
    assert_eq!(
        session.map_data().string_table().get_by_id(s3.id),
        Some(b"Different String".as_slice())
    );
}
