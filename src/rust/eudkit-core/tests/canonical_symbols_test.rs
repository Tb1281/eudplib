use eudkit_core::canonical::{
    stock_unit_id, CanonicalSymbols, CANONICAL_UNITS, CANONICAL_UNIT_NAMES,
};

#[test]
fn canonical_symbols_resolve_without_a_map_context() {
    let symbols = CanonicalSymbols::new();

    assert_eq!(symbols.unit_names().len(), 228);
    assert_eq!(symbols.units().len(), 228);
    assert_eq!(symbols.unit_name(0), Some("Terran Marine"));
    assert_eq!(symbols.unit_name(37), Some("Zerg Zergling"));
    assert_eq!(symbols.unit_name(227), Some("Vespene Tank (Terran Type 2)"));
    assert_eq!(symbols.unit_name(228), None);

    for unit in CANONICAL_UNITS {
        assert_eq!(symbols.unit_id(unit.name), Some(unit.id));
        assert_eq!(symbols.unit_name(unit.id), Some(unit.name));
        assert_eq!(CANONICAL_UNIT_NAMES[unit.id as usize], unit.name);
        for alias in unit.aliases {
            assert_eq!(symbols.unit_id(alias), Some(unit.id));
            assert_eq!(stock_unit_id(alias), Some(unit.id));
        }
    }
}

#[test]
fn canonical_symbols_cover_aliases_and_unit_list_filters() {
    let symbols = CanonicalSymbols::new();

    assert_eq!(symbols.unit_id("Dark Templar"), Some(74));
    assert_eq!(symbols.unit_id("Protoss Dark Templar (Hero)"), Some(74));
    assert_eq!(symbols.unit_id("Tank Turret type   1"), Some(6));
    assert_eq!(symbols.unit_id("(any unit)"), Some(229));
    assert_eq!(symbols.unit_id("Men"), Some(230));
    assert_eq!(symbols.unit_id("(buildings)"), Some(231));
    assert_eq!(symbols.unit_id("Factories"), Some(232));
    assert_eq!(symbols.unit_id("None"), Some(228));
    assert_eq!(symbols.unit_id("Not a unit"), None);

    for name in ["Dark Templar", "Tank Turret type   1", "(factories)"] {
        assert_eq!(stock_unit_id(name), symbols.unit_id(name));
    }
}

#[test]
fn canonical_location_and_switch_names_do_not_need_map_data() {
    let symbols = CanonicalSymbols::new();

    assert_eq!(symbols.location_id("Anywhere"), Some(64));
    assert_eq!(symbols.location_id("Location 1"), Some(1));
    assert_eq!(symbols.location_id("Location 64"), None);
    assert_eq!(symbols.location_id("Location 255"), Some(255));
    assert_eq!(symbols.location_id("Location 256"), None);
    assert_eq!(symbols.location_id("Anywhere Else"), None);

    assert_eq!(symbols.switch_id("Switch 1"), Some(0));
    assert_eq!(symbols.switch_id("Switch 256"), Some(255));
    assert_eq!(symbols.switch_id("Switch 257"), None);
    assert_eq!(symbols.switch_id("Switch 0"), None);
}
