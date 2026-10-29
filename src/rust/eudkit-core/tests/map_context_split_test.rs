use eudkit_core::canonical::{CanonicalSymbols, CANONICAL_UNITS};
use eudkit_core::chk::Chk;
use eudkit_core::map_context::{MapData, MapSymbol};
use eudkit_core::{dts, symbols, CompilationConfig, CompilationSession};
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

fn basemap_session() -> CompilationSession {
    let map_path = repo_root().join("tests/outputmap/basemap/basemap_strx.scx");
    CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load basemap_strx.scx")
}

#[test]
fn map_data_owns_map_symbols_and_delegates_canonical_names() {
    let session = basemap_session();
    let data = session.map_data();
    let canonical = session.canonical_symbols();

    // Map Data resolves a name that actually exists in MRGN.
    let named_location = data
        .locations()
        .iter()
        .find(|location| location.name.is_some())
        .expect("basemap should have a named location");
    let named_location_id = named_location.id;
    let named_location_name = named_location.name.as_deref().unwrap();
    assert_eq!(
        data.location_symbol(named_location_name),
        MapSymbol::Unique(named_location_id)
    );
    assert_eq!(
        symbols::location_id(data, canonical, named_location_name),
        Some(named_location_id)
    );

    // Map Data has no canonical fallback. Find a stock unit the map does not name.
    let (stock_unit, stock_id) = CANONICAL_UNITS
        .iter()
        .find(|unit| data.unit_symbol(unit.name) == MapSymbol::Missing)
        .map(|unit| (unit.name, unit.id))
        .expect("basemap should not custom-name every stock unit");
    assert_eq!(data.unit_symbol(stock_unit), MapSymbol::Missing);
    assert_eq!(canonical.unit_id(stock_unit), Some(stock_id));
    assert_eq!(
        symbols::unit_id(data, canonical, stock_unit),
        Some(stock_id)
    );

    // Map Data reports unknown names as Missing, not as a canonical fallback.
    assert_eq!(
        data.location_symbol("__eudkit_missing_location__"),
        MapSymbol::Missing
    );
    assert_eq!(
        data.switch_symbol("__eudkit_missing_switch__"),
        MapSymbol::Missing
    );
    assert_eq!(
        data.unit_symbol("__eudkit_missing_unit__"),
        MapSymbol::Missing
    );

    // The resolver composes Map Symbols first and Canonical Symbols second.
    assert_eq!(symbols::location_id(data, canonical, "Anywhere"), Some(64));
    assert_eq!(symbols::switch_id(data, canonical, "Switch 1"), Some(0));
}

#[test]
fn one_resolver_policy_prefers_unique_map_symbols_over_canonical_symbols() {
    // The map names the same three canonical names at different IDs.
    let data = map_data_with_names(
        &["Anywhere", "Terran Marine", "Switch 2"],
        &[1],
        &[(5, 2)],
        &[(7, 3)],
    );
    let canonical = CanonicalSymbols::new();

    assert_eq!(data.location_symbol("Anywhere"), MapSymbol::Unique(1));
    assert_eq!(symbols::location_id(&data, &canonical, "Anywhere"), Some(1));
    assert_eq!(data.unit_symbol("Terran Marine"), MapSymbol::Unique(5));
    assert_eq!(
        symbols::unit_id(&data, &canonical, "Terran Marine"),
        Some(5)
    );
    assert_eq!(data.switch_symbol("Switch 2"), MapSymbol::Unique(7));
    assert_eq!(symbols::switch_id(&data, &canonical, "Switch 2"), Some(7));
}

#[test]
fn ambiguous_map_symbols_do_not_fall_back_to_canonical_symbols() {
    // Each name is ambiguous in Map Data and also a valid Canonical Symbol.
    let data = map_data_with_names(
        &["Anywhere", "Terran Marine", "Switch 1"],
        &[1, 1],
        &[(0, 2), (1, 2)],
        &[(0, 3), (1, 3)],
    );
    let canonical = CanonicalSymbols::new();

    assert_eq!(data.location_symbol("Anywhere"), MapSymbol::Ambiguous);
    assert_eq!(symbols::location_id(&data, &canonical, "Anywhere"), None);
    assert_eq!(data.unit_symbol("Terran Marine"), MapSymbol::Ambiguous);
    assert_eq!(symbols::unit_id(&data, &canonical, "Terran Marine"), None);
    assert_eq!(data.switch_symbol("Switch 1"), MapSymbol::Ambiguous);
    assert_eq!(symbols::switch_id(&data, &canonical, "Switch 1"), None);
}

#[test]
fn missing_map_symbols_fall_back_to_canonical_symbols() {
    let data = map_data_with_names(&[], &[], &[], &[]);
    let canonical = CanonicalSymbols::new();

    assert_eq!(data.location_symbol("Anywhere"), MapSymbol::Missing);
    assert_eq!(
        symbols::location_id(&data, &canonical, "Anywhere"),
        Some(64)
    );
    assert_eq!(data.unit_symbol("Terran Marine"), MapSymbol::Missing);
    assert_eq!(
        symbols::unit_id(&data, &canonical, "Terran Marine"),
        Some(0)
    );
    assert_eq!(data.switch_symbol("Switch 2"), MapSymbol::Missing);
    assert_eq!(symbols::switch_id(&data, &canonical, "Switch 2"), Some(1));
}

#[test]
fn map_data_and_canonical_symbols_emit_dts_directly() {
    let session = basemap_session();
    let direct = dts::generate_dts(session.map_data(), session.canonical_symbols(), "basemap");

    assert!(direct.contains("// Generated by eudkit for basemap"));
    assert!(direct.contains("\"Terran Marine\""));
    assert!(direct.contains("\"Zerg Zergling\""));
    assert!(direct.contains("export declare const mapContext: MapContext;"));
}

/// Builds Map Data from a minimal CHK containing only the Map Symbol sources.
fn map_data_with_names(
    strings: &[&str],
    location_name_string_ids: &[u16],
    unit_names: &[(usize, u16)],
    switch_names: &[(usize, u32)],
) -> MapData {
    let mut chk = Chk::new();
    chk.set_section(b"STRx", strx_section(strings)).unwrap();
    chk.set_section(b"MRGN", mrgn_section(location_name_string_ids))
        .unwrap();
    chk.set_section(b"UNIx", unix_section(unit_names)).unwrap();
    chk.set_section(b"SWNM", swnm_section(switch_names))
        .unwrap();
    MapData::from_chk(&chk).expect("minimal CHK should parse as Map Data")
}

fn strx_section(strings: &[&str]) -> Vec<u8> {
    let count = strings.len();
    let table_size = 4 * (count + 1);
    let mut offset = table_size;
    let mut offsets = Vec::with_capacity(count);
    for text in strings {
        offsets.push(offset as u32);
        offset += roundup_by_4(text.len() + 1);
    }

    let mut out = Vec::with_capacity(offset);
    out.extend_from_slice(&(count as u32).to_le_bytes());
    for string_offset in offsets {
        out.extend_from_slice(&string_offset.to_le_bytes());
    }
    for text in strings {
        out.extend_from_slice(text.as_bytes());
        out.resize(out.len() + (roundup_by_4(text.len() + 1) - text.len()), 0);
    }
    out
}

fn mrgn_section(location_name_string_ids: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(location_name_string_ids.len() * 20);
    for (index, name_string_id) in location_name_string_ids.iter().enumerate() {
        out.extend_from_slice(&(index as i32).to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes());
        out.extend_from_slice(&(index as i32 + 1).to_le_bytes());
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&name_string_id.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

fn unix_section(unit_names: &[(usize, u16)]) -> Vec<u8> {
    let mut out = vec![0u8; 3648];
    for (unit_id, name_string_id) in unit_names {
        let offset = 3192 + *unit_id * 2;
        out[offset..offset + 2].copy_from_slice(&name_string_id.to_le_bytes());
    }
    out
}

fn swnm_section(switch_names: &[(usize, u32)]) -> Vec<u8> {
    let mut out = vec![0u8; 256 * 4];
    for (switch_id, name_string_id) in switch_names {
        let offset = *switch_id * 4;
        out[offset..offset + 4].copy_from_slice(&name_string_id.to_le_bytes());
    }
    out
}

fn roundup_by_4(value: usize) -> usize {
    (value + 3) & !3
}
