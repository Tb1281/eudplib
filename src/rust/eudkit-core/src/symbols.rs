//! One resolution policy for Map Symbols and Canonical Symbols.
//!
//! A Frontend names a location, unit, or switch. Resolution is map-first: a
//! unique Map Symbol from the Map Data wins, an ambiguous Map Symbol is an
//! error rather than a guess, and only a missing Map Symbol falls back to the
//! Canonical Symbols that StarCraft defines without a Map Context.

use crate::canonical::CanonicalSymbols;
use crate::map_context::{MapData, MapSymbol};

/// Resolves a unit name through Map Symbols first, then Canonical Symbols.
///
/// Returns `None` when the Map Data contains more than one unit with the name;
/// an ambiguous Map Symbol never falls back to a Canonical Symbol.
pub fn unit_id(map: &MapData, canonical: &CanonicalSymbols, name: &str) -> Option<u16> {
    resolve(map.unit_symbol(name), || canonical.unit_id(name))
}

/// Resolves a location name through Map Symbols first, then Canonical Symbols.
///
/// Returns `None` when the Map Data contains more than one location with the
/// name; an ambiguous Map Symbol never falls back to a Canonical Symbol.
pub fn location_id(map: &MapData, canonical: &CanonicalSymbols, name: &str) -> Option<u32> {
    resolve(map.location_symbol(name), || canonical.location_id(name))
}

/// Resolves a switch name through Map Symbols first, then Canonical Symbols.
///
/// Returns `None` when the Map Data contains more than one switch with the
/// name; an ambiguous Map Symbol never falls back to a Canonical Symbol.
pub fn switch_id(map: &MapData, canonical: &CanonicalSymbols, name: &str) -> Option<u8> {
    resolve(map.switch_symbol(name), || canonical.switch_id(name))
}

/// Applies the single Map Symbol / Canonical Symbol precedence rule.
fn resolve<T>(map_symbol: MapSymbol<T>, canonical: impl FnOnce() -> Option<T>) -> Option<T> {
    match map_symbol {
        MapSymbol::Unique(id) => Some(id),
        MapSymbol::Ambiguous => None,
        MapSymbol::Missing => canonical(),
    }
}
