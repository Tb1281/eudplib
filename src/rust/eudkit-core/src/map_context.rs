pub use crate::canonical::{
    stock_unit_id, CanonicalSymbols, CanonicalUnit, CANONICAL_UNITS, CANONICAL_UNIT_NAMES,
};
use crate::chk::Chk;
use crate::chk_layout::classic;
use crate::error::{EudkitError, Result};
use std::collections::{HashMap, HashSet};

fn roundup_by_4(num: usize) -> usize {
    (num + 3) & !3
}

fn strip_color_codes(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut has_color = false;
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        if (0x01..=0x1F).contains(&b) || b == 0x7F {
            has_color = true;
        } else {
            out.push(b);
        }
    }
    if has_color {
        Some(out)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationInfo {
    /// 1-based location ID (`1..=255`, where `64` is `"Anywhere"`).
    pub id: u32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub name_string_id: u16,
    pub name: Option<String>,
    pub elevation_flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedUnit {
    pub serial: u32,
    pub x: u16,
    pub y: u16,
    pub unit_id: u16,
    pub relation_flags: u16,
    pub valid_special_flags: u16,
    pub valid_unit_flags: u16,
    pub owner: u8,
    pub hp_percent: u8,
    pub shield_percent: u8,
    pub energy_percent: u8,
    pub resource_amount: u32,
    pub hangar_count: u16,
    pub special_state_flags: u16,
    pub related_serial: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuwpSlot {
    /// 1-based CUWP slot index (`1..=64`).
    pub id: u8,
    pub in_use: bool,
    pub sprp_valid: u16,
    pub prp_valid: u16,
    pub player: u8,
    pub hitpoint: u8,
    pub shield: u8,
    pub energy: u8,
    pub resource: u32,
    pub hangar: u16,
    pub sprp_flag: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSetting {
    /// 0-based unit ID (`0..228`).
    pub id: u16,
    pub use_default: bool,
    pub hitpoints: u32,
    pub shields: u16,
    pub armor: u8,
    pub build_time: u16,
    pub mineral_cost: u16,
    pub gas_cost: u16,
    pub name_string_id: u16,
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchInfo {
    /// 0-based switch ID (`0..256`).
    pub id: u8,
    pub name_string_id: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTrigger {
    pub index: usize,
    pub flags: u32,
    pub players: [u8; 28],
    pub condition_count: usize,
    pub action_count: usize,
    pub raw: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerSlot {
    pub slot: u8,
    pub owner: u8,
    pub side: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScenarioProperties {
    pub scenario_name_string_id: u16,
    pub scenario_description_string_id: u16,
    pub force_assignments: [u8; 8],
    pub force_name_string_ids: [u16; 4],
    pub force_flags: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapString {
    pub id: u32,
    pub text: String,
    pub offset: u32,
    pub address: u32,
    pub epd: i32,
}

#[derive(Debug, Clone, Default)]
pub struct StringTable {
    datatb: Vec<Vec<u8>>,
    stringmap: HashMap<Vec<u8>, usize>,
    dataindextb: Vec<usize>,
    emptystring: Vec<(usize, Option<Vec<u8>>)>,
    loaded: bool,
    first_extended_string: Option<Vec<u8>>,
}

impl StringTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_with_chk(
        content: &[u8],
        entry_size: usize,
        removed_str_ids: &HashSet<usize>,
        raw_strings_out: &mut Vec<Vec<u8>>,
    ) -> Result<Self> {
        let mut tbl = Self::new();
        if content.len() < entry_size {
            tbl.loaded = true;
            return Ok(tbl);
        }

        let read_entry = |offset: usize| -> Option<usize> {
            if offset + entry_size > content.len() {
                return None;
            }
            Some(if entry_size == 2 {
                u16::from_le_bytes(content[offset..offset + 2].try_into().unwrap()) as usize
            } else {
                u32::from_le_bytes(content[offset..offset + 4].try_into().unwrap()) as usize
            })
        };

        let string_count = read_entry(0).unwrap_or(0);
        raw_strings_out.reserve(string_count);

        for idx0 in 0..string_count {
            let id1 = idx0 + 1;
            let Some(string_offset) = read_entry(id1 * entry_size) else {
                break;
            };
            if string_offset >= content.len() {
                raw_strings_out.push(Vec::new());
                continue;
            }
            let mut send = string_offset;
            while send < content.len() && content[send] != 0 {
                send += 1;
            }
            let raw_str = content[string_offset..send].to_vec();
            raw_strings_out.push(raw_str.clone());

            let is_removed = removed_str_ids.contains(&id1);
            if raw_str.is_empty() || is_removed {
                let mut found_next = None;
                for j0 in id1..string_count {
                    let j1 = j0 + 1;
                    if removed_str_ids.contains(&j1) {
                        continue;
                    }
                    let Some(next_off) = read_entry(j1 * entry_size) else {
                        break;
                    };
                    if next_off >= content.len() {
                        continue;
                    }
                    let mut next_end = next_off;
                    while next_end < content.len() && content[next_end] != 0 {
                        next_end += 1;
                    }
                    let next_str = &content[next_off..next_end];
                    if !next_str.is_empty() {
                        found_next = Some(next_str.to_vec());
                        break;
                    }
                }
                tbl.emptystring.push((id1 - 1, found_next));
            }

            let effective = if is_removed { Vec::new() } else { raw_str };
            tbl.add_string_zero_based(&effective);
        }

        tbl.loaded = true;
        Ok(tbl)
    }

    fn add_string_zero_based(&mut self, s: &[u8]) -> usize {
        let mut string_index = self.dataindextb.len();
        if let Some(&repr_id) = self.stringmap.get(s) {
            let data_index = self.dataindextb[repr_id];
            self.dataindextb.push(data_index);
        } else {
            if !self.emptystring.is_empty() && self.loaded && self.first_extended_string.is_some() {
                let (empty_idx, next_opt) = self.emptystring.remove(0);
                string_index = empty_idx;
                let target_next =
                    next_opt.unwrap_or_else(|| self.first_extended_string.clone().unwrap());
                let data_index = self
                    .datatb
                    .iter()
                    .position(|x| *x == target_next)
                    .unwrap_or(self.datatb.len());
                self.datatb.insert(data_index, s.to_vec());
                for v in &mut self.dataindextb {
                    if *v >= data_index {
                        *v += 1;
                    }
                }
                self.dataindextb[string_index] = data_index;
            } else {
                if self.first_extended_string.is_none() {
                    self.first_extended_string = Some(s.to_vec());
                }
                let data_index = self.datatb.len();
                self.datatb.push(s.to_vec());
                self.dataindextb.push(data_index);
            }
            self.stringmap.insert(s.to_vec(), string_index);
        }
        string_index
    }

    /// Returns the 1-based string ID (`1..=N`) for `s`, inserting it if not present.
    pub fn get_or_insert(&mut self, s: &[u8]) -> u32 {
        if let Some(&idx0) = self.stringmap.get(s) {
            (idx0 + 1) as u32
        } else {
            (self.add_string_zero_based(s) + 1) as u32
        }
    }

    /// Looks up the 1-based string ID (`1..=N`) for `s` without inserting.
    pub fn find_id(&self, s: &[u8]) -> Option<u32> {
        self.stringmap.get(s).map(|&idx0| (idx0 + 1) as u32)
    }

    /// Gets string bytes by 1-based string ID (`1..=N`).
    pub fn get_by_id(&self, id1: u32) -> Option<&[u8]> {
        if id1 == 0 {
            return None;
        }
        let data_idx = *self.dataindextb.get((id1 - 1) as usize)?;
        self.datatb.get(data_idx).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.dataindextb.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dataindextb.is_empty()
    }

    pub fn datatb(&self) -> &[Vec<u8>] {
        &self.datatb
    }

    pub fn datatb_mut(&mut self) -> &mut [Vec<u8>] {
        &mut self.datatb
    }

    pub fn data_index_offset(&self, data_idx: usize) -> Option<u32> {
        let count = self.dataindextb.len();
        let table_size = 4 * (count + 1);
        let mut out_index = roundup_by_4(table_size);
        for (i, s) in self.datatb.iter().enumerate() {
            if i == data_idx {
                return Some(out_index as u32);
            }
            out_index += roundup_by_4(s.len() + 1);
        }
        None
    }

    /// Returns the byte offset of string `id1` within the serialized STRx section.
    pub fn string_offset(&self, id1: u32) -> Option<u32> {
        if id1 == 0 || (id1 as usize) > self.dataindextb.len() {
            return None;
        }
        let data_idx = *self.dataindextb.get((id1 - 1) as usize)?;
        let count = self.dataindextb.len();
        let table_size = 4 * (count + 1);
        let mut out_index = roundup_by_4(table_size);
        for (i, s) in self.datatb.iter().enumerate() {
            if i == data_idx {
                return Some(out_index as u32);
            }
            out_index += roundup_by_4(s.len() + 1);
        }
        None
    }

    /// Returns the virtual memory address in StarCraft for string `id1` (`0x191943C8 + offset`).
    pub fn string_address(&self, id1: u32) -> Option<u32> {
        self.string_offset(id1)
            .map(|off| crate::hir::STRX_BASE_ADDR.wrapping_add(off))
    }

    /// Returns the EPD address for string `id1`.
    pub fn string_epd(&self, id1: u32) -> Option<i32> {
        self.string_address(id1).map(|addr| crate::hir::epd(addr))
    }

    /// Serializes into classic `STR` format (16-bit offsets) for map editor compatibility.
    pub fn serialize_str(&self) -> Vec<u8> {
        let count = self.dataindextb.len().min(65535);
        let table_size = 2 * (count + 1);
        let mut out_index = table_size;
        let mut str_offsets = Vec::with_capacity(self.datatb.len());

        for s in &self.datatb {
            if out_index + s.len() + 1 > 65535 {
                str_offsets.push(0u16);
            } else {
                str_offsets.push(out_index as u16);
                out_index += s.len() + 1;
            }
        }

        let mut out = Vec::with_capacity(out_index.min(65535));
        out.extend_from_slice(&(count as u16).to_le_bytes());
        for &data_idx in &self.dataindextb[..count] {
            let off = str_offsets.get(data_idx).copied().unwrap_or(0);
            out.extend_from_slice(&off.to_le_bytes());
        }

        for s in &self.datatb {
            if out.len() + s.len() + 1 > 65535 {
                break;
            }
            out.extend_from_slice(s);
            out.push(0);
        }
        out
    }

    /// Serializes into Remastered `STRx` format (32-bit offsets, 4-byte aligned entries).
    pub fn serialize_strx(&self) -> Vec<u8> {
        let count = self.dataindextb.len();
        let table_size = 4 * (count + 1);
        let mut out_index = roundup_by_4(table_size);
        let mut str_offsets = Vec::with_capacity(self.datatb.len());

        for s in &self.datatb {
            str_offsets.push(out_index as u32);
            out_index += roundup_by_4(s.len() + 1);
        }

        let mut out = Vec::with_capacity(out_index);
        out.extend_from_slice(&(count as u32).to_le_bytes());
        for &data_idx in &self.dataindextb {
            out.extend_from_slice(&str_offsets[data_idx].to_le_bytes());
        }
        out.resize(roundup_by_4(table_size), 0);

        for s in &self.datatb {
            out.extend_from_slice(s);
            let padded = roundup_by_4(s.len() + 1);
            out.resize(out.len() + (padded - s.len()), 0);
        }
        out
    }
}

#[derive(Debug, Clone, Default)]
struct SymbolMap<T: Copy> {
    entries: HashMap<Vec<u8>, Option<T>>,
}

impl<T: Copy> SymbolMap<T> {
    fn insert(&mut self, key: Vec<u8>, value: T) {
        self.entries
            .entry(key)
            .and_modify(|v| *v = None)
            .or_insert(Some(value));
    }

    fn get(&self, key: &[u8]) -> Option<Option<T>> {
        self.entries.get(key).copied()
    }
}

/// Resolution state of one Map Symbol name in a [`MapData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSymbol<T> {
    /// The name is not present in the map.
    Missing,
    /// The map contains more than one asset with this name.
    Ambiguous,
    /// The map contains exactly one asset with this name.
    Unique(T),
}

fn resolve_symbol<T: Copy>(map: &SymbolMap<T>, key: &[u8]) -> MapSymbol<T> {
    match map.get(key) {
        None => MapSymbol::Missing,
        Some(None) => MapSymbol::Ambiguous,
        Some(Some(value)) => MapSymbol::Unique(value),
    }
}

/// The read-only assets and Map Symbols extracted from one input `.scx`/`.scm`.
///
/// `MapData` owns only map-dependent state. Canonical names such as stock units
/// and `"Anywhere"` live in [`CanonicalSymbols`], and TypeScript emission lives
/// in [`crate::dts`].
#[derive(Debug, Clone)]
pub struct MapData {
    strings: Vec<Vec<u8>>,
    string_table: StringTable,
    locations: Vec<LocationInfo>,
    location_lookup: SymbolMap<u32>,
    placed_units: Vec<PlacedUnit>,
    cuwp_slots: Vec<CuwpSlot>,
    unit_settings: Vec<UnitSetting>,
    unit_lookup: SymbolMap<u16>,
    switches: Vec<SwitchInfo>,
    switch_lookup: SymbolMap<u8>,
    triggers: Vec<MapTrigger>,
    players: [PlayerSlot; 12],
    scenario_properties: ScenarioProperties,
}

impl MapData {
    pub fn from_chk(chk: &Chk) -> Result<Self> {
        let mut loc_str_to_idx0: HashMap<usize, usize> = HashMap::new();
        let mut sw_str_to_idx0: HashMap<usize, usize> = HashMap::new();
        let mut unit_str_to_idx0: HashMap<usize, usize> = HashMap::new();
        let mut reserved_str: HashSet<usize> = HashSet::new();

        if let Some(mrgn) = chk.section(b"MRGN") {
            let loc_count = mrgn.len() / 20;
            for i in 0..loc_count {
                let loc_str_id =
                    u16::from_le_bytes(mrgn[20 * i + 16..20 * i + 18].try_into().unwrap()) as usize;
                if loc_str_id != 0 {
                    loc_str_to_idx0.insert(loc_str_id, i);
                }
            }
        }

        if let Some(swnm) = chk.section(b"SWNM") {
            let sw_count = (swnm.len() / 4).min(256);
            for i in 0..sw_count {
                let sw_str_id =
                    u32::from_le_bytes(swnm[4 * i..4 * i + 4].try_into().unwrap()) as usize;
                if sw_str_id != 0 {
                    sw_str_to_idx0.insert(sw_str_id, i);
                }
            }
        }

        if let Some(unix) = chk.section(b"UNIx") {
            if unix.len() >= 3192 + 228 * 2 {
                for i in 0..228 {
                    let off = 3192 + i * 2;
                    let unit_str_id =
                        u16::from_le_bytes(unix[off..off + 2].try_into().unwrap()) as usize;
                    if unit_str_id != 0 {
                        unit_str_to_idx0.insert(unit_str_id, i);
                        reserved_str.insert(unit_str_id);
                    }
                }
            }
        }

        if let Some(sprp) = chk.section(b"SPRP") {
            if sprp.len() >= 4 {
                for i in 0..2 {
                    let sid =
                        u16::from_le_bytes(sprp[i * 2..i * 2 + 2].try_into().unwrap()) as usize;
                    if sid != 0 {
                        reserved_str.insert(sid);
                    }
                }
            }
        }

        if let Some(forc) = chk.section(b"FORC") {
            if forc.len() >= 16 {
                for i in 0..4 {
                    let off = 8 + i * 2;
                    let sid = u16::from_le_bytes(forc[off..off + 2].try_into().unwrap()) as usize;
                    if sid != 0 {
                        reserved_str.insert(sid);
                    }
                }
            }
        }

        let mut removed_str: HashSet<usize> = HashSet::new();
        for &sid in loc_str_to_idx0.keys().chain(sw_str_to_idx0.keys()) {
            if !reserved_str.contains(&sid) {
                removed_str.insert(sid);
            }
        }

        let mut raw_strings = Vec::new();
        let string_table = if let Some(strx) = chk.section(b"STRx") {
            StringTable::load_with_chk(strx, 4, &removed_str, &mut raw_strings)?
        } else if let Some(str_sec) = chk.section(b"STR ") {
            StringTable::load_with_chk(str_sec, 2, &removed_str, &mut raw_strings)?
        } else {
            return Err(EudkitError::MissingSection("STR / STRx".to_string()));
        };

        let raw_string_at = |id1: usize| -> Option<&[u8]> {
            if id1 == 0 {
                None
            } else {
                raw_strings
                    .get(id1 - 1)
                    .map(Vec::as_slice)
                    .filter(|s| !s.is_empty())
            }
        };

        // Parse MRGN (Locations)
        let mut locations = Vec::new();
        let mut location_lookup = SymbolMap::default();
        if let Some(mrgn) = chk.section(b"MRGN") {
            let loc_count = mrgn.len() / 20;
            locations.reserve(loc_count);
            for i in 0..loc_count {
                let base = 20 * i;
                let left = i32::from_le_bytes(mrgn[base..base + 4].try_into().unwrap());
                let top = i32::from_le_bytes(mrgn[base + 4..base + 8].try_into().unwrap());
                let right = i32::from_le_bytes(mrgn[base + 8..base + 12].try_into().unwrap());
                let bottom = i32::from_le_bytes(mrgn[base + 12..base + 16].try_into().unwrap());
                let name_string_id =
                    u16::from_le_bytes(mrgn[base + 16..base + 18].try_into().unwrap());
                let elevation_flags =
                    u16::from_le_bytes(mrgn[base + 18..base + 20].try_into().unwrap());
                let id1 = (i + 1) as u32;
                let name = raw_string_at(name_string_id as usize).map(|bytes| {
                    location_lookup.insert(bytes.to_vec(), id1);
                    String::from_utf8_lossy(bytes).into_owned()
                });
                locations.push(LocationInfo {
                    id: id1,
                    left,
                    top,
                    right,
                    bottom,
                    name_string_id,
                    name,
                    elevation_flags,
                });
            }
        }

        // Parse UNIT (Placed Units, 36B each)
        let mut placed_units = Vec::new();
        if let Some(unit) = chk.section(b"UNIT") {
            for rec in unit.chunks_exact(36) {
                placed_units.push(PlacedUnit {
                    serial: u32::from_le_bytes(rec[0..4].try_into().unwrap()),
                    x: u16::from_le_bytes(rec[4..6].try_into().unwrap()),
                    y: u16::from_le_bytes(rec[6..8].try_into().unwrap()),
                    unit_id: u16::from_le_bytes(rec[8..10].try_into().unwrap()),
                    relation_flags: u16::from_le_bytes(rec[10..12].try_into().unwrap()),
                    valid_special_flags: u16::from_le_bytes(rec[12..14].try_into().unwrap()),
                    valid_unit_flags: u16::from_le_bytes(rec[14..16].try_into().unwrap()),
                    owner: rec[16],
                    hp_percent: rec[17],
                    shield_percent: rec[18],
                    energy_percent: rec[19],
                    resource_amount: u32::from_le_bytes(rec[20..24].try_into().unwrap()),
                    hangar_count: u16::from_le_bytes(rec[24..26].try_into().unwrap()),
                    special_state_flags: u16::from_le_bytes(rec[26..28].try_into().unwrap()),
                    related_serial: u32::from_le_bytes(rec[32..36].try_into().unwrap()),
                });
            }
        }

        // Parse UPRP & UPUS (64 CUWP slots, 20B each)
        let mut cuwp_slots = Vec::with_capacity(64);
        let upus = chk.section(b"UPUS");
        if let Some(uprp) = chk.section(b"UPRP") {
            for (i, rec) in uprp.chunks_exact(20).take(64).enumerate() {
                let in_use = upus.and_then(|u| u.get(i)).map(|&v| v != 0).unwrap_or(true);
                cuwp_slots.push(CuwpSlot {
                    id: (i + 1) as u8,
                    in_use,
                    sprp_valid: u16::from_le_bytes(rec[0..2].try_into().unwrap()),
                    prp_valid: u16::from_le_bytes(rec[2..4].try_into().unwrap()),
                    player: rec[4],
                    hitpoint: rec[5],
                    shield: rec[6],
                    energy: rec[7],
                    resource: u32::from_le_bytes(rec[8..12].try_into().unwrap()),
                    hangar: u16::from_le_bytes(rec[12..14].try_into().unwrap()),
                    sprp_flag: u16::from_le_bytes(rec[14..16].try_into().unwrap()),
                });
            }
        }

        // Parse UNIx (228 units)
        let mut unit_settings = Vec::with_capacity(228);
        let mut unit_lookup = SymbolMap::default();
        if let Some(unix) = chk.section(b"UNIx") {
            if unix.len() >= 3648 {
                for i in 0..228 {
                    let use_default = unix[i] != 0;
                    let hp_off = 228 + i * 4;
                    let sh_off = 1140 + i * 2;
                    let ar_off = 1596 + i;
                    let bt_off = 1824 + i * 2;
                    let min_off = 2280 + i * 2;
                    let gas_off = 2736 + i * 2;
                    let str_off = 3192 + i * 2;

                    let hitpoints =
                        u32::from_le_bytes(unix[hp_off..hp_off + 4].try_into().unwrap());
                    let shields = u16::from_le_bytes(unix[sh_off..sh_off + 2].try_into().unwrap());
                    let armor = unix[ar_off];
                    let build_time =
                        u16::from_le_bytes(unix[bt_off..bt_off + 2].try_into().unwrap());
                    let mineral_cost =
                        u16::from_le_bytes(unix[min_off..min_off + 2].try_into().unwrap());
                    let gas_cost =
                        u16::from_le_bytes(unix[gas_off..gas_off + 2].try_into().unwrap());
                    let name_string_id =
                        u16::from_le_bytes(unix[str_off..str_off + 2].try_into().unwrap());

                    let unit_id = i as u16;
                    let custom_name = raw_string_at(name_string_id as usize).map(|bytes| {
                        unit_lookup.insert(bytes.to_vec(), unit_id);
                        if let Some(stripped) = strip_color_codes(bytes) {
                            unit_lookup.insert(stripped, unit_id);
                        }
                        String::from_utf8_lossy(bytes).into_owned()
                    });

                    unit_settings.push(UnitSetting {
                        id: unit_id,
                        use_default,
                        hitpoints,
                        shields,
                        armor,
                        build_time,
                        mineral_cost,
                        gas_cost,
                        name_string_id,
                        custom_name,
                    });
                }
            }
        }

        // Parse SWNM (256 switches)
        let mut switches = Vec::with_capacity(256);
        let mut switch_lookup = SymbolMap::default();
        if let Some(swnm) = chk.section(b"SWNM") {
            for (i, rec) in swnm.chunks_exact(4).take(256).enumerate() {
                let name_string_id = u32::from_le_bytes(rec.try_into().unwrap());
                let sw_id = i as u8;
                let name = raw_string_at(name_string_id as usize).map(|bytes| {
                    switch_lookup.insert(bytes.to_vec(), sw_id);
                    String::from_utf8_lossy(bytes).into_owned()
                });
                switches.push(SwitchInfo {
                    id: sw_id,
                    name_string_id,
                    name,
                });
            }
        } else {
            for i in 0..=255u8 {
                switches.push(SwitchInfo {
                    id: i,
                    name_string_id: 0,
                    name: None,
                });
            }
        }

        // Parse classic TRIG entries.
        let mut triggers = Vec::new();
        if let Some(trig) = chk.section(b"TRIG") {
            for (idx, rec) in trig.chunks_exact(classic::TRIGGER_SIZE).enumerate() {
                let trigger = classic::unpack_trigger(rec);
                triggers.push(MapTrigger {
                    index: idx,
                    flags: trigger.flags(),
                    players: trigger.players(),
                    condition_count: trigger.condition_count(),
                    action_count: trigger.action_count(),
                    raw: rec.to_vec(),
                });
            }
        }

        // Parse OWNR and SIDE (12 player slots)
        let mut players = [PlayerSlot::default(); 12];
        let ownr = chk.section(b"OWNR");
        let side = chk.section(b"SIDE");
        for i in 0..12 {
            players[i] = PlayerSlot {
                slot: i as u8,
                owner: ownr.and_then(|o| o.get(i)).copied().unwrap_or(0),
                side: side.and_then(|s| s.get(i)).copied().unwrap_or(0),
            };
        }

        // Parse SPRP and FORC
        let mut scenario_properties = ScenarioProperties::default();
        if let Some(sprp) = chk.section(b"SPRP") {
            if sprp.len() >= 4 {
                scenario_properties.scenario_name_string_id =
                    u16::from_le_bytes(sprp[0..2].try_into().unwrap());
                scenario_properties.scenario_description_string_id =
                    u16::from_le_bytes(sprp[2..4].try_into().unwrap());
            }
        }
        if let Some(forc) = chk.section(b"FORC") {
            let assign_len = forc.len().min(8);
            scenario_properties.force_assignments[..assign_len]
                .copy_from_slice(&forc[..assign_len]);
            if forc.len() >= 16 {
                for i in 0..4 {
                    let off = 8 + i * 2;
                    scenario_properties.force_name_string_ids[i] =
                        u16::from_le_bytes(forc[off..off + 2].try_into().unwrap());
                }
            }
            if forc.len() >= 20 {
                scenario_properties
                    .force_flags
                    .copy_from_slice(&forc[16..20]);
            }
        }

        Ok(Self {
            strings: raw_strings,
            string_table,
            locations,
            location_lookup,
            placed_units,
            cuwp_slots,
            unit_settings,
            unit_lookup,
            switches,
            switch_lookup,
            triggers,
            players,
            scenario_properties,
        })
    }

    pub fn strings(&self) -> &[Vec<u8>] {
        &self.strings
    }

    pub fn string_table(&self) -> &StringTable {
        &self.string_table
    }

    pub fn string_table_mut(&mut self) -> &mut StringTable {
        &mut self.string_table
    }

    pub fn get_or_insert_string(&mut self, s: impl AsRef<[u8]>) -> u32 {
        self.string_table.get_or_insert(s.as_ref())
    }

    /// Registers a string in the STRx string table, deduplicating with existing entries,
    /// and resolves its static offset, virtual address, and EPD without post-collection callbacks.
    pub fn register_map_string(&mut self, text: &str) -> Result<MapString> {
        let id = self.string_table.get_or_insert(text.as_bytes());
        let offset = self.string_table.string_offset(id).ok_or_else(|| {
            EudkitError::Ir(format!("failed to compute offset for string ID {id}"))
        })?;
        let address = crate::hir::STRX_BASE_ADDR.wrapping_add(offset);
        let epd = crate::hir::epd(address);
        Ok(MapString {
            id,
            text: text.to_string(),
            offset,
            address,
            epd,
        })
    }

    pub fn locations(&self) -> &[LocationInfo] {
        &self.locations
    }

    /// Resolves a location name through the map's own MRGN names.
    ///
    /// Canonical fallbacks such as `"Anywhere"` and `"Location N"` belong to
    /// [`CanonicalSymbols::location_id`].
    pub fn location_symbol(&self, name: &str) -> MapSymbol<u32> {
        resolve_symbol(&self.location_lookup, name.as_bytes())
    }

    pub fn placed_units(&self) -> &[PlacedUnit] {
        &self.placed_units
    }

    pub fn cuwp_slots(&self) -> &[CuwpSlot] {
        &self.cuwp_slots
    }

    pub fn unit_settings(&self) -> &[UnitSetting] {
        &self.unit_settings
    }

    /// Resolves a unit name through the map's custom `UNIx` names.
    ///
    /// Stock unit names and aliases belong to [`CanonicalSymbols::unit_id`].
    pub fn unit_symbol(&self, name: &str) -> MapSymbol<u16> {
        resolve_symbol(&self.unit_lookup, name.as_bytes())
    }

    pub fn switches(&self) -> &[SwitchInfo] {
        &self.switches
    }

    /// Resolves a switch name through the map's own SWNM names.
    ///
    /// Canonical `"Switch N"` names belong to [`CanonicalSymbols::switch_id`].
    pub fn switch_symbol(&self, name: &str) -> MapSymbol<u8> {
        resolve_symbol(&self.switch_lookup, name.as_bytes())
    }

    pub fn triggers(&self) -> &[MapTrigger] {
        &self.triggers
    }

    pub fn players(&self) -> &[PlayerSlot; 12] {
        &self.players
    }

    pub fn scenario_properties(&self) -> &ScenarioProperties {
        &self.scenario_properties
    }
}

/// Compatibility facade over [`MapData`] and [`CanonicalSymbols`].
///
/// New code should compose the two modules directly. This facade keeps the
/// existing `CompilationSession::map_context` interface working for one
/// release; it will be removed after downstream callers migrate.
#[derive(Debug, Clone)]
pub struct MapContext {
    data: MapData,
    canonical: CanonicalSymbols,
}

impl MapContext {
    pub fn from_chk(chk: &Chk) -> Result<Self> {
        Ok(Self {
            data: MapData::from_chk(chk)?,
            canonical: CanonicalSymbols::new(),
        })
    }

    /// Composes a Map Context from already-parsed Map Data and Canonical Symbols.
    pub fn from_parts(data: MapData, canonical: CanonicalSymbols) -> Self {
        Self { data, canonical }
    }

    /// Returns the map-dependent half of the Map Context.
    pub fn data(&self) -> &MapData {
        &self.data
    }

    /// Returns the mutable map-dependent half of the Map Context.
    pub fn data_mut(&mut self) -> &mut MapData {
        &mut self.data
    }

    /// Returns the canonical half of the Map Context.
    pub fn canonical_symbols(&self) -> &CanonicalSymbols {
        &self.canonical
    }

    pub fn strings(&self) -> &[Vec<u8>] {
        self.data.strings()
    }

    pub fn string_table(&self) -> &StringTable {
        self.data.string_table()
    }

    pub fn string_table_mut(&mut self) -> &mut StringTable {
        self.data.string_table_mut()
    }

    pub fn get_or_insert_string(&mut self, s: impl AsRef<[u8]>) -> u32 {
        self.data.get_or_insert_string(s)
    }

    pub fn register_map_string(&mut self, text: &str) -> Result<MapString> {
        self.data.register_map_string(text)
    }

    pub fn locations(&self) -> &[LocationInfo] {
        self.data.locations()
    }

    /// Resolves a location name through Map Symbols first, then Canonical Symbols.
    pub fn location_id(&self, name: &str) -> Option<u32> {
        match self.data.location_symbol(name) {
            MapSymbol::Unique(id) => Some(id),
            MapSymbol::Ambiguous => None,
            MapSymbol::Missing => self.canonical.location_id(name),
        }
    }

    pub fn placed_units(&self) -> &[PlacedUnit] {
        self.data.placed_units()
    }

    pub fn cuwp_slots(&self) -> &[CuwpSlot] {
        self.data.cuwp_slots()
    }

    pub fn unit_settings(&self) -> &[UnitSetting] {
        self.data.unit_settings()
    }

    /// Resolves a unit name through Map Symbols first, then Canonical Symbols.
    pub fn unit_id(&self, name: &str) -> Option<u16> {
        match self.data.unit_symbol(name) {
            MapSymbol::Unique(id) => Some(id),
            MapSymbol::Ambiguous => None,
            MapSymbol::Missing => self.canonical.unit_id(name),
        }
    }

    pub fn switches(&self) -> &[SwitchInfo] {
        self.data.switches()
    }

    /// Resolves a switch name through Map Symbols first, then Canonical Symbols.
    pub fn switch_id(&self, name: &str) -> Option<u8> {
        match self.data.switch_symbol(name) {
            MapSymbol::Unique(id) => Some(id),
            MapSymbol::Ambiguous => None,
            MapSymbol::Missing => self.canonical.switch_id(name),
        }
    }

    pub fn triggers(&self) -> &[MapTrigger] {
        self.data.triggers()
    }

    pub fn players(&self) -> &[PlayerSlot; 12] {
        self.data.players()
    }

    pub fn scenario_properties(&self) -> &ScenarioProperties {
        self.data.scenario_properties()
    }

    /// Generates a TypeScript `.d.ts` file from the two halves of the Map Context.
    pub fn generate_dts(&self, module_name: &str) -> String {
        crate::dts::generate_dts(&self.data, &self.canonical, module_name)
    }
}
