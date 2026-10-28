use crate::chk::Chk;
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

#[derive(Debug, Clone)]
pub struct MapContext {
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

impl MapContext {
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

        // Parse TRIG (2400B each)
        let mut triggers = Vec::new();
        if let Some(trig) = chk.section(b"TRIG") {
            for (idx, rec) in trig.chunks_exact(2400).enumerate() {
                let flags = u32::from_le_bytes(rec[2368..2372].try_into().unwrap());
                let mut players = [0u8; 28];
                players.copy_from_slice(&rec[2372..2400]);

                let mut condition_count = 0;
                for c in 0..16 {
                    let cond_type = rec[c * 20 + 15];
                    if cond_type == 0 {
                        break;
                    }
                    condition_count += 1;
                }

                let mut action_count = 0;
                for a in 0..64 {
                    let act_type = rec[320 + a * 32 + 26];
                    if act_type == 0 {
                        break;
                    }
                    action_count += 1;
                }

                triggers.push(MapTrigger {
                    index: idx,
                    flags,
                    players,
                    condition_count,
                    action_count,
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

    /// Resolves a location name to its 1-based StarCraft location ID (`1..=255`).
    pub fn location_id(&self, name: &str) -> Option<u32> {
        if let Some(opt_id) = self.location_lookup.get(name.as_bytes()) {
            return opt_id;
        }
        if name == "Anywhere" {
            return Some(64);
        }
        if let Some(rest) = name.strip_prefix("Location ") {
            if let Ok(idx) = rest.parse::<u32>() {
                if (1..=255).contains(&idx) && idx != 64 {
                    return Some(idx);
                }
            }
        }
        None
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

    /// Resolves a unit name (custom `UNIx` name or stock StarCraft unit name) to its 0-based unit ID.
    pub fn unit_id(&self, name: &str) -> Option<u16> {
        if let Some(opt_id) = self.unit_lookup.get(name.as_bytes()) {
            return opt_id;
        }
        stock_unit_id(name)
    }

    pub fn switches(&self) -> &[SwitchInfo] {
        &self.switches
    }

    /// Resolves a switch name to its 0-based switch ID (`0..=255`).
    pub fn switch_id(&self, name: &str) -> Option<u8> {
        if let Some(opt_id) = self.switch_lookup.get(name.as_bytes()) {
            return opt_id;
        }
        if let Some(rest) = name.strip_prefix("Switch ") {
            if let Ok(idx1) = rest.parse::<u16>() {
                if (1..=256).contains(&idx1) {
                    return Some((idx1 - 1) as u8);
                }
            }
        }
        None
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

    /// Generates TypeScript `.d.ts` declaration file containing typed Map Context definitions:
    /// - Location names and IDs
    /// - Unit names and IDs (stock units and custom UNIx units)
    /// - Switch names and IDs
    /// - Player IDs
    /// - String table records
    pub fn generate_dts(&self, module_name: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "// Generated by eudkit for {module_name}. Do not edit manually.\n\n"
        ));

        // 1. Locations
        let mut loc_map: Vec<(String, u32)> = Vec::new();
        loc_map.push(("Anywhere".to_string(), 64));
        for loc in &self.locations {
            if let Some(name) = &loc.name {
                let trimmed = name.trim();
                if !trimmed.is_empty() && trimmed != "Anywhere" {
                    loc_map.push((trimmed.to_string(), loc.id));
                }
            }
        }
        for i in 1..=255u32 {
            if i != 64 {
                loc_map.push((format!("Location {i}"), i));
            }
        }

        // Deduplicate names while keeping first occurrence
        let mut seen_locs = HashSet::new();
        loc_map.retain(|(name, _)| seen_locs.insert(name.clone()));

        out.push_str("export type LocationId =\n");
        for (name, _) in &loc_map {
            out.push_str(&format!("  | {:?}\n", name));
        }
        out.push_str("  | number;\n\n");

        out.push_str("export const Locations = {\n");
        for (name, id) in &loc_map {
            out.push_str(&format!("  {:?}: {},\n", name, id));
        }
        out.push_str("} as const;\n\n");

        // 2. Units
        let mut unit_map: Vec<(String, u16)> = Vec::new();
        for (idx, &name) in CANONICAL_UNIT_NAMES.iter().enumerate() {
            unit_map.push((name.to_string(), idx as u16));
        }
        for u in &self.unit_settings {
            if let Some(custom) = &u.custom_name {
                let trimmed = custom.trim();
                if !trimmed.is_empty() {
                    unit_map.push((trimmed.to_string(), u.id));
                }
            }
        }
        let mut seen_units = HashSet::new();
        unit_map.retain(|(name, _)| seen_units.insert(name.clone()));

        out.push_str("export type UnitId =\n");
        for (name, _) in &unit_map {
            out.push_str(&format!("  | {:?}\n", name));
        }
        out.push_str("  | number;\n\n");

        out.push_str("export const Units = {\n");
        for (name, id) in &unit_map {
            out.push_str(&format!("  {:?}: {},\n", name, id));
        }
        out.push_str("} as const;\n\n");

        // 3. Switches
        let mut switch_map: Vec<(String, u8)> = Vec::new();
        for sw in &self.switches {
            if let Some(name) = &sw.name {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    switch_map.push((trimmed.to_string(), sw.id));
                }
            }
        }
        for i in 1..=256u16 {
            switch_map.push((format!("Switch {i}"), (i - 1) as u8));
        }
        let mut seen_switches = HashSet::new();
        switch_map.retain(|(name, _)| seen_switches.insert(name.clone()));

        out.push_str("export type SwitchId =\n");
        for (name, _) in &switch_map {
            out.push_str(&format!("  | {:?}\n", name));
        }
        out.push_str("  | number;\n\n");

        out.push_str("export const Switches = {\n");
        for (name, id) in &switch_map {
            out.push_str(&format!("  {:?}: {},\n", name, id));
        }
        out.push_str("} as const;\n\n");

        // 4. Players
        out.push_str(
            r#"export type PlayerId =
  | "P1" | "P2" | "P3" | "P4" | "P5" | "P6" | "P7" | "P8"
  | "P9" | "P10" | "P11" | "P12"
  | "Player1" | "Player2" | "Player3" | "Player4" | "Player5" | "Player6"
  | "Player7" | "Player8" | "Player9" | "Player10" | "Player11" | "Player12"
  | "CurrentPlayer"
  | "Foes"
  | "Allies"
  | "NeutralPlayers"
  | "AllPlayers"
  | "Force1" | "Force2" | "Force3" | "Force4"
  | "NonAlliedVictoryPlayers"
  | number;

export const Players = {
  P1: 0,
  P2: 1,
  P3: 2,
  P4: 3,
  P5: 4,
  P6: 5,
  P7: 6,
  P8: 7,
  P9: 8,
  P10: 9,
  P11: 10,
  P12: 11,
  Player1: 0,
  Player2: 1,
  Player3: 2,
  Player4: 3,
  Player5: 4,
  Player6: 5,
  Player7: 6,
  Player8: 7,
  Player9: 8,
  Player10: 9,
  Player11: 10,
  Player12: 11,
  CurrentPlayer: 13,
  Foes: 14,
  Allies: 15,
  NeutralPlayers: 16,
  AllPlayers: 17,
  Force1: 18,
  Force2: 19,
  Force3: 20,
  Force4: 21,
  NonAlliedVictoryPlayers: 26,
} as const;

export type StringId = number;

export interface MapContext {
  readonly locations: Record<string, number>;
  readonly units: Record<string, number>;
  readonly switches: Record<string, number>;
  readonly strings: Record<number, string>;
}

export declare const mapContext: MapContext;
"#,
        );

        out
    }
}

pub const CANONICAL_UNIT_NAMES: [&str; 228] = [
    "Terran Marine",
    "Terran Ghost",
    "Terran Vulture",
    "Terran Goliath",
    "Goliath Turret",
    "Terran Siege Tank (Tank Mode)",
    "Siege Tank Turret (Tank Mode)",
    "Terran SCV",
    "Terran Wraith",
    "Terran Science Vessel",
    "Gui Montag",
    "Terran Dropship",
    "Terran Battlecruiser",
    "Spider Mine",
    "Nuclear Missile",
    "Terran Civilian",
    "Sarah Kerrigan",
    "Alan Schezar",
    "Alan Schezar Turret",
    "Jim Raynor (Vulture)",
    "Jim Raynor (Marine)",
    "Tom Kazansky",
    "Magellan",
    "Edmund Duke (Tank Mode)",
    "Edmund Duke Turret (Tank Mode)",
    "Edmund Duke (Siege Mode)",
    "Edmund Duke Turret (Siege Mode)",
    "Arcturus Mengsk",
    "Hyperion",
    "Norad II (Battlecruiser)",
    "Terran Siege Tank (Siege Mode)",
    "Siege Tank Turret (Siege Mode)",
    "Terran Firebat",
    "Scanner Sweep",
    "Terran Medic",
    "Zerg Larva",
    "Zerg Egg",
    "Zerg Zergling",
    "Zerg Hydralisk",
    "Zerg Ultralisk",
    "Zerg Broodling",
    "Zerg Drone",
    "Zerg Overlord",
    "Zerg Mutalisk",
    "Zerg Guardian",
    "Zerg Queen",
    "Zerg Defiler",
    "Zerg Scourge",
    "Torrasque",
    "Matriarch",
    "Infested Terran",
    "Infested Kerrigan",
    "Unclean One",
    "Hunter Killer",
    "Devouring One",
    "Kukulza (Mutalisk)",
    "Kukulza (Guardian)",
    "Yggdrasill",
    "Terran Valkyrie",
    "Mutalisk Cocoon",
    "Protoss Corsair",
    "Protoss Dark Templar",
    "Zerg Devourer",
    "Protoss Dark Archon",
    "Protoss Probe",
    "Protoss Zealot",
    "Protoss Dragoon",
    "Protoss High Templar",
    "Protoss Archon",
    "Protoss Shuttle",
    "Protoss Scout",
    "Protoss Arbiter",
    "Protoss Carrier",
    "Protoss Interceptor",
    "Dark Templar (Hero)",
    "Zeratul",
    "Tassadar/Zeratul",
    "Fenix (Zealot)",
    "Fenix (Dragoon)",
    "Tassadar",
    "Mojo",
    "Warbringer",
    "Gantrithor",
    "Protoss Reaver",
    "Protoss Observer",
    "Protoss Scarab",
    "Danimoth",
    "Aldaris",
    "Artanis",
    "Rhynadon",
    "Bengalaas",
    "Cargo Ship",
    "Mercenary Gunship",
    "Scantid",
    "Kakaru",
    "Ragnasaur",
    "Ursadon",
    "Lurker Egg",
    "Raszagal",
    "Samir Duran",
    "Alexei Stukov",
    "Map Revealer",
    "Gerard DuGalle",
    "Zerg Lurker",
    "Infested Duran",
    "Disruption Web",
    "Terran Command Center",
    "Terran Comsat Station",
    "Terran Nuclear Silo",
    "Terran Supply Depot",
    "Terran Refinery",
    "Terran Barracks",
    "Terran Academy",
    "Terran Factory",
    "Terran Starport",
    "Terran Control Tower",
    "Terran Science Facility",
    "Terran Covert Ops",
    "Terran Physics Lab",
    "Starbase",
    "Terran Machine Shop",
    "Repair Bay",
    "Terran Engineering Bay",
    "Terran Armory",
    "Terran Missile Turret",
    "Terran Bunker",
    "Norad II (Crashed)",
    "Ion Cannon",
    "Uraj Crystal",
    "Khalis Crystal",
    "Infested Command Center",
    "Zerg Hatchery",
    "Zerg Lair",
    "Zerg Hive",
    "Zerg Nydus Canal",
    "Zerg Hydralisk Den",
    "Zerg Defiler Mound",
    "Zerg Greater Spire",
    "Zerg Queen's Nest",
    "Zerg Evolution Chamber",
    "Zerg Ultralisk Cavern",
    "Zerg Spire",
    "Zerg Spawning Pool",
    "Zerg Creep Colony",
    "Zerg Spore Colony",
    "Unused Zerg Bldg",
    "Zerg Sunken Colony",
    "Zerg Overmind (With Shell)",
    "Zerg Overmind",
    "Zerg Extractor",
    "Mature Chrysalis",
    "Zerg Cerebrate",
    "Zerg Cerebrate Daggoth",
    "Unused Zerg Building2",
    "Protoss Nexus",
    "Protoss Robotics Facility",
    "Protoss Pylon",
    "Protoss Assimilator",
    "Protoss Unused type   1",
    "Protoss Observatory",
    "Protoss Gateway",
    "Protoss Unused type   2",
    "Protoss Photon Cannon",
    "Protoss Citadel of Adun",
    "Protoss Cybernetics Core",
    "Protoss Templar Archives",
    "Protoss Forge",
    "Protoss Stargate",
    "Stasis Cell/Prison",
    "Protoss Fleet Beacon",
    "Protoss Arbiter Tribunal",
    "Protoss Robotics Support Bay",
    "Protoss Shield Battery",
    "Khaydarin Crystal Formation",
    "Protoss Temple",
    "Xel'Naga Temple",
    "Mineral Field (Type 1)",
    "Mineral Field (Type 2)",
    "Mineral Field (Type 3)",
    "Cave",
    "Cave-in",
    "Cantina",
    "Mining Platform",
    "Independent Command Center",
    "Independent Starport",
    "Jump Gate",
    "Ruins",
    "Kyadarin Crystal Formation",
    "Vespene Geyser",
    "Warp Gate",
    "Psi Disrupter",
    "Zerg Marker",
    "Terran Marker",
    "Protoss Marker",
    "Zerg Beacon",
    "Terran Beacon",
    "Protoss Beacon",
    "Zerg Flag Beacon",
    "Terran Flag Beacon",
    "Protoss Flag Beacon",
    "Power Generator",
    "Overmind Cocoon",
    "Dark Swarm",
    "Floor Missile Trap",
    "Floor Hatch (Unused)",
    "Left Upper Level Door",
    "Right Upper Level Door",
    "Left Pit Door",
    "Right Pit Door",
    "Floor Gun Trap",
    "Left Wall Missile Trap",
    "Left Wall Flame Trap",
    "Right Wall Missile Trap",
    "Right Wall Flame Trap",
    "Start Location",
    "Flag",
    "Young Chrysalis",
    "Psi Emitter",
    "Data Disc",
    "Khaydarin Crystal",
    "Mineral Chunk (Type 1)",
    "Mineral Chunk (Type 2)",
    "Vespene Orb (Protoss Type 1)",
    "Vespene Orb (Protoss Type 2)",
    "Vespene Sac (Zerg Type 1)",
    "Vespene Sac (Zerg Type 2)",
    "Vespene Tank (Terran Type 1)",
    "Vespene Tank (Terran Type 2)",
];

pub fn stock_unit_id(name: &str) -> Option<u16> {
    let id = match name {
        "Terran Marine" => 0,
        "Terran Ghost" => 1,
        "Terran Vulture" => 2,
        "Terran Goliath" => 3,
        "Goliath Turret" => 4,
        "Terran Siege Tank (Tank Mode)" => 5,
        "Siege Tank Turret (Tank Mode)" | "Tank Turret type   1" => 6,
        "Terran SCV" => 7,
        "Terran Wraith" => 8,
        "Terran Science Vessel" => 9,
        "Gui Montag" | "Gui Montag (Firebat)" => 10,
        "Terran Dropship" => 11,
        "Terran Battlecruiser" => 12,
        "Spider Mine" | "Vulture Spider Mine" => 13,
        "Nuclear Missile" => 14,
        "Terran Civilian" => 15,
        "Sarah Kerrigan" | "Sarah Kerrigan (Ghost)" => 16,
        "Alan Schezar" | "Alan Schezar (Goliath)" => 17,
        "Alan Schezar Turret" | "Alan Turret" => 18,
        "Jim Raynor (Vulture)" => 19,
        "Jim Raynor (Marine)" => 20,
        "Tom Kazansky" | "Tom Kazansky (Wraith)" => 21,
        "Magellan" | "Magellan (Science Vessel)" => 22,
        "Edmund Duke (Tank Mode)" => 23,
        "Edmund Duke Turret (Tank Mode)" | "Duke Turret type   1" => 24,
        "Edmund Duke (Siege Mode)" => 25,
        "Edmund Duke Turret (Siege Mode)" | "Duke Turret type   2" => 26,
        "Arcturus Mengsk" | "Arcturus Mengsk (Battlecruiser)" => 27,
        "Hyperion" | "Hyperion (Battlecruiser)" => 28,
        "Norad II (Battlecruiser)" => 29,
        "Terran Siege Tank (Siege Mode)" => 30,
        "Siege Tank Turret (Siege Mode)" | "Tank Turret type   2" => 31,
        "Terran Firebat" => 32,
        "Scanner Sweep" => 33,
        "Terran Medic" => 34,
        "Zerg Larva" => 35,
        "Zerg Egg" => 36,
        "Zerg Zergling" => 37,
        "Zerg Hydralisk" => 38,
        "Zerg Ultralisk" => 39,
        "Zerg Broodling" => 40,
        "Zerg Drone" => 41,
        "Zerg Overlord" => 42,
        "Zerg Mutalisk" => 43,
        "Zerg Guardian" => 44,
        "Zerg Queen" => 45,
        "Zerg Defiler" => 46,
        "Zerg Scourge" => 47,
        "Torrasque" | "Torrasque (Ultralisk)" => 48,
        "Matriarch" | "Matriarch (Queen)" => 49,
        "Infested Terran" => 50,
        "Infested Kerrigan" | "Infested Kerrigan (Infested Terran)" => 51,
        "Unclean One" | "Unclean One (Defiler)" => 52,
        "Hunter Killer" | "Hunter Killer (Hydralisk)" => 53,
        "Devouring One" | "Devouring One (Zergling)" => 54,
        "Kukulza (Mutalisk)" => 55,
        "Kukulza (Guardian)" => 56,
        "Yggdrasill" | "Yggdrasill (Overlord)" => 57,
        "Terran Valkyrie" => 58,
        "Mutalisk Cocoon" | "Cocoon" => 59,
        "Protoss Corsair" => 60,
        "Protoss Dark Templar" | "Protoss Dark Templar (Unit)" => 61,
        "Zerg Devourer" => 62,
        "Protoss Dark Archon" => 63,
        "Protoss Probe" => 64,
        "Protoss Zealot" => 65,
        "Protoss Dragoon" => 66,
        "Protoss High Templar" => 67,
        "Protoss Archon" => 68,
        "Protoss Shuttle" => 69,
        "Protoss Scout" => 70,
        "Protoss Arbiter" => 71,
        "Protoss Carrier" => 72,
        "Protoss Interceptor" => 73,
        "Dark Templar" | "Dark Templar (Hero)" | "Protoss Dark Templar (Hero)" => 74,
        "Zeratul" | "Zeratul (Dark Templar)" => 75,
        "Tassadar/Zeratul" | "Tassadar/Zeratul (Archon)" => 76,
        "Fenix (Zealot)" => 77,
        "Fenix (Dragoon)" => 78,
        "Tassadar" | "Tassadar (Templar)" => 79,
        "Mojo" | "Mojo (Scout)" => 80,
        "Warbringer" | "Warbringer (Reaver)" => 81,
        "Gantrithor" | "Gantrithor (Carrier)" => 82,
        "Protoss Reaver" => 83,
        "Protoss Observer" => 84,
        "Protoss Scarab" => 85,
        "Danimoth" | "Danimoth (Arbiter)" => 86,
        "Aldaris" | "Aldaris (Templar)" => 87,
        "Artanis" | "Artanis (Scout)" => 88,
        "Rhynadon" | "Rhynadon (Badlands)" | "Rhynadon (Badlands Critter)" => 89,
        "Bengalaas" | "Bengalaas (Jungle)" | "Bengalaas (Jungle Critter)" => 90,
        "Cargo Ship" | "Cargo Ship (Unused)" | "Unused type   1" => 91,
        "Unused type   2" | "Mercenary Gunship" | "Mercenary Gunship (Unused)" => 92,
        "Scantid" | "Scantid (Desert)" | "Scantid (Desert Critter)" => 93,
        "Kakaru" | "Kakaru (Twilight)" | "Kakaru (Twilight Critter)" => 94,
        "Ragnasaur" | "Ragnasaur (Ashworld)" | "Ragnasaur (Ashworld Critter)" => 95,
        "Ursadon" | "Ursadon (Ice World)" | "Ursadon (Ice World Critter)" => 96,
        "Lurker Egg" | "Zerg Lurker Egg" => 97,
        "Raszagal" | "Raszagal (Corsair)" => 98,
        "Samir Duran" | "Samir Duran (Ghost)" => 99,
        "Alexei Stukov" | "Alexei Stukov (Ghost)" => 100,
        "Map Revealer" => 101,
        "Gerard DuGalle" | "Gerard DuGalle (Ghost)" | "Gerard DuGalle (BattleCruiser)" => 102,
        "Zerg Lurker" => 103,
        "Infested Duran" | "Infested Duran (Infested Terran)" => 104,
        "Disruption Web" | "Disruption Field" => 105,
        "Terran Command Center" => 106,
        "Terran Comsat Station" => 107,
        "Terran Nuclear Silo" => 108,
        "Terran Supply Depot" => 109,
        "Terran Refinery" => 110,
        "Terran Barracks" => 111,
        "Terran Academy" => 112,
        "Terran Factory" => 113,
        "Terran Starport" => 114,
        "Terran Control Tower" => 115,
        "Terran Science Facility" => 116,
        "Terran Covert Ops" => 117,
        "Terran Physics Lab" => 118,
        "Starbase" | "Starbase (Unused)" | "Unused Terran Bldg type   1" => 119,
        "Terran Machine Shop" => 120,
        "Repair Bay" | "Repair Bay (Unused)" | "Unused Terran Bldg type   2" => 121,
        "Terran Engineering Bay" => 122,
        "Terran Armory" => 123,
        "Terran Missile Turret" => 124,
        "Terran Bunker" => 125,
        "Norad II (Crashed)" | "Norad II (Crashed Battlecruiser)" => 126,
        "Ion Cannon" => 127,
        "Uraj Crystal" => 128,
        "Khalis Crystal" => 129,
        "Infested Command Center" => 130,
        "Zerg Hatchery" => 131,
        "Zerg Lair" => 132,
        "Zerg Hive" => 133,
        "Zerg Nydus Canal" => 134,
        "Zerg Hydralisk Den" => 135,
        "Zerg Defiler Mound" => 136,
        "Zerg Greater Spire" => 137,
        "Zerg Queen's Nest" => 138,
        "Zerg Evolution Chamber" => 139,
        "Zerg Ultralisk Cavern" => 140,
        "Zerg Spire" => 141,
        "Zerg Spawning Pool" => 142,
        "Zerg Creep Colony" => 143,
        "Zerg Spore Colony" => 144,
        "Unused Zerg Bldg" | "Unused Zerg Building1" => 145,
        "Zerg Sunken Colony" => 146,
        "Zerg Overmind (With Shell)" => 147,
        "Zerg Overmind" => 148,
        "Zerg Extractor" => 149,
        "Mature Chrysalis" => 150,
        "Zerg Cerebrate" => 151,
        "Zerg Cerebrate Daggoth" => 152,
        "Unused Zerg Building2" => 153,
        "Protoss Nexus" => 154,
        "Protoss Robotics Facility" => 155,
        "Protoss Pylon" => 156,
        "Protoss Assimilator" => 157,
        "Protoss Unused type   1" | "Unused Protoss Building1" => 158,
        "Protoss Observatory" => 159,
        "Protoss Gateway" => 160,
        "Protoss Unused type   2" | "Unused Protoss Building2" => 161,
        "Protoss Photon Cannon" => 162,
        "Protoss Citadel of Adun" => 163,
        "Protoss Cybernetics Core" => 164,
        "Protoss Templar Archives" => 165,
        "Protoss Forge" => 166,
        "Protoss Stargate" => 167,
        "Stasis Cell/Prison" => 168,
        "Protoss Fleet Beacon" => 169,
        "Protoss Arbiter Tribunal" => 170,
        "Protoss Robotics Support Bay" => 171,
        "Protoss Shield Battery" => 172,
        "Khaydarin Crystal Formation" => 173,
        "Protoss Temple" => 174,
        "Xel'Naga Temple" => 175,
        "Mineral Field (Type 1)" => 176,
        "Mineral Field (Type 2)" => 177,
        "Mineral Field (Type 3)" => 178,
        "Cave" | "Cave (Unused)" => 179,
        "Cave-in" | "Cave-in (Unused)" => 180,
        "Cantina" | "Cantina (Unused)" => 181,
        "Mining Platform" | "Mining Platform (Unused)" => 182,
        "Independent Command Center" | "Independent Command Center (Unused)" => 183,
        "Independent Starport" | "Independent Starport (Unused)" => 184,
        "Jump Gate" | "Independent Jump Gate (Unused)" => 185,
        "Ruins" | "Ruins (Unused)" => 186,
        "Kyadarin Crystal Formation" | "Khaydarin Crystal Formation (Unused)" => 187,
        "Vespene Geyser" => 188,
        "Warp Gate" => 189,
        "Psi Disrupter" => 190,
        "Zerg Marker" => 191,
        "Terran Marker" => 192,
        "Protoss Marker" => 193,
        "Zerg Beacon" => 194,
        "Terran Beacon" => 195,
        "Protoss Beacon" => 196,
        "Zerg Flag Beacon" => 197,
        "Terran Flag Beacon" => 198,
        "Protoss Flag Beacon" => 199,
        "Power Generator" => 200,
        "Overmind Cocoon" => 201,
        "Dark Swarm" => 202,
        "Floor Missile Trap" => 203,
        "Floor Hatch (Unused)" | "Floor Hatch (UNUSED)" => 204,
        "Left Upper Level Door" => 205,
        "Right Upper Level Door" => 206,
        "Left Pit Door" => 207,
        "Right Pit Door" => 208,
        "Floor Gun Trap" => 209,
        "Left Wall Missile Trap" => 210,
        "Left Wall Flame Trap" => 211,
        "Right Wall Missile Trap" => 212,
        "Right Wall Flame Trap" => 213,
        "Start Location" => 214,
        "Flag" => 215,
        "Young Chrysalis" => 216,
        "Psi Emitter" => 217,
        "Data Disc" => 218,
        "Khaydarin Crystal" => 219,
        "Mineral Chunk (Type 1)" | "Mineral Cluster Type 1" => 220,
        "Mineral Chunk (Type 2)" | "Mineral Cluster Type 2" => 221,
        "Vespene Orb (Protoss Type 1)" | "Protoss Vespene Gas Orb Type 1" => 222,
        "Vespene Orb (Protoss Type 2)" | "Protoss Vespene Gas Orb Type 2" => 223,
        "Vespene Sac (Zerg Type 1)" | "Zerg Vespene Gas Sac Type 1" => 224,
        "Vespene Sac (Zerg Type 2)" | "Zerg Vespene Gas Sac Type 2" => 225,
        "Vespene Tank (Terran Type 1)" | "Terran Vespene Gas Tank Type 1" => 226,
        "Vespene Tank (Terran Type 2)" | "Terran Vespene Gas Tank Type 2" => 227,
        "Unused unit 228" | "Unused228" | "None" => 228,
        "Any unit" | "(any unit)" => 229,
        "Men" | "(men)" => 230,
        "Buildings" | "(buildings)" => 231,
        "Factories" | "(factories)" => 232,
        _ => return None,
    };
    Some(id)
}
