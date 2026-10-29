//! Canonical Symbols: StarCraft-defined names available without a Map Context.
//!
//! This module owns the single table of stock unit names, their aliases, and
//! their IDs, plus the canonical location and switch fallbacks. [`MapContext`]
//! composes these symbols with [`MapData`](crate::map_context::MapData), so a
//! caller that only needs canonical names depends on this module alone.

use std::collections::HashMap;

/// One stock unit and every name that resolves to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalUnit {
    /// The 0-based StarCraft unit ID.
    pub id: u16,
    /// The canonical name, also used as the ordered list entry for this ID.
    pub name: &'static str,
    /// Alternate spellings that resolve to the same unit ID.
    pub aliases: &'static [&'static str],
}

macro_rules! canonical_units {
    ($( $id:literal => $name:literal $(| $alias:literal)* ),* $(,)?) => {
        /// The 228 concrete stock units, ordered by unit ID.
        pub const CANONICAL_UNITS: [CanonicalUnit; 228] = [
            $(CanonicalUnit {
                id: $id,
                name: $name,
                aliases: &[$($alias),*],
            }),*
        ];

        /// Canonical stock unit names indexed by unit ID (`0..=227`).
        pub const CANONICAL_UNIT_NAMES: [&str; 228] = [ $($name),* ];
    };
}

canonical_units! {
    0 => "Terran Marine",
    1 => "Terran Ghost",
    2 => "Terran Vulture",
    3 => "Terran Goliath",
    4 => "Goliath Turret",
    5 => "Terran Siege Tank (Tank Mode)",
    6 => "Siege Tank Turret (Tank Mode)" | "Tank Turret type   1",
    7 => "Terran SCV",
    8 => "Terran Wraith",
    9 => "Terran Science Vessel",
    10 => "Gui Montag" | "Gui Montag (Firebat)",
    11 => "Terran Dropship",
    12 => "Terran Battlecruiser",
    13 => "Spider Mine" | "Vulture Spider Mine",
    14 => "Nuclear Missile",
    15 => "Terran Civilian",
    16 => "Sarah Kerrigan" | "Sarah Kerrigan (Ghost)",
    17 => "Alan Schezar" | "Alan Schezar (Goliath)",
    18 => "Alan Schezar Turret" | "Alan Turret",
    19 => "Jim Raynor (Vulture)",
    20 => "Jim Raynor (Marine)",
    21 => "Tom Kazansky" | "Tom Kazansky (Wraith)",
    22 => "Magellan" | "Magellan (Science Vessel)",
    23 => "Edmund Duke (Tank Mode)",
    24 => "Edmund Duke Turret (Tank Mode)" | "Duke Turret type   1",
    25 => "Edmund Duke (Siege Mode)",
    26 => "Edmund Duke Turret (Siege Mode)" | "Duke Turret type   2",
    27 => "Arcturus Mengsk" | "Arcturus Mengsk (Battlecruiser)",
    28 => "Hyperion" | "Hyperion (Battlecruiser)",
    29 => "Norad II (Battlecruiser)",
    30 => "Terran Siege Tank (Siege Mode)",
    31 => "Siege Tank Turret (Siege Mode)" | "Tank Turret type   2",
    32 => "Terran Firebat",
    33 => "Scanner Sweep",
    34 => "Terran Medic",
    35 => "Zerg Larva",
    36 => "Zerg Egg",
    37 => "Zerg Zergling",
    38 => "Zerg Hydralisk",
    39 => "Zerg Ultralisk",
    40 => "Zerg Broodling",
    41 => "Zerg Drone",
    42 => "Zerg Overlord",
    43 => "Zerg Mutalisk",
    44 => "Zerg Guardian",
    45 => "Zerg Queen",
    46 => "Zerg Defiler",
    47 => "Zerg Scourge",
    48 => "Torrasque" | "Torrasque (Ultralisk)",
    49 => "Matriarch" | "Matriarch (Queen)",
    50 => "Infested Terran",
    51 => "Infested Kerrigan" | "Infested Kerrigan (Infested Terran)",
    52 => "Unclean One" | "Unclean One (Defiler)",
    53 => "Hunter Killer" | "Hunter Killer (Hydralisk)",
    54 => "Devouring One" | "Devouring One (Zergling)",
    55 => "Kukulza (Mutalisk)",
    56 => "Kukulza (Guardian)",
    57 => "Yggdrasill" | "Yggdrasill (Overlord)",
    58 => "Terran Valkyrie",
    59 => "Mutalisk Cocoon" | "Cocoon",
    60 => "Protoss Corsair",
    61 => "Protoss Dark Templar" | "Protoss Dark Templar (Unit)",
    62 => "Zerg Devourer",
    63 => "Protoss Dark Archon",
    64 => "Protoss Probe",
    65 => "Protoss Zealot",
    66 => "Protoss Dragoon",
    67 => "Protoss High Templar",
    68 => "Protoss Archon",
    69 => "Protoss Shuttle",
    70 => "Protoss Scout",
    71 => "Protoss Arbiter",
    72 => "Protoss Carrier",
    73 => "Protoss Interceptor",
    74 => "Dark Templar (Hero)" | "Dark Templar" | "Protoss Dark Templar (Hero)",
    75 => "Zeratul" | "Zeratul (Dark Templar)",
    76 => "Tassadar/Zeratul" | "Tassadar/Zeratul (Archon)",
    77 => "Fenix (Zealot)",
    78 => "Fenix (Dragoon)",
    79 => "Tassadar" | "Tassadar (Templar)",
    80 => "Mojo" | "Mojo (Scout)",
    81 => "Warbringer" | "Warbringer (Reaver)",
    82 => "Gantrithor" | "Gantrithor (Carrier)",
    83 => "Protoss Reaver",
    84 => "Protoss Observer",
    85 => "Protoss Scarab",
    86 => "Danimoth" | "Danimoth (Arbiter)",
    87 => "Aldaris" | "Aldaris (Templar)",
    88 => "Artanis" | "Artanis (Scout)",
    89 => "Rhynadon" | "Rhynadon (Badlands)" | "Rhynadon (Badlands Critter)",
    90 => "Bengalaas" | "Bengalaas (Jungle)" | "Bengalaas (Jungle Critter)",
    91 => "Cargo Ship" | "Cargo Ship (Unused)" | "Unused type   1",
    92 => "Mercenary Gunship" | "Unused type   2" | "Mercenary Gunship (Unused)",
    93 => "Scantid" | "Scantid (Desert)" | "Scantid (Desert Critter)",
    94 => "Kakaru" | "Kakaru (Twilight)" | "Kakaru (Twilight Critter)",
    95 => "Ragnasaur" | "Ragnasaur (Ashworld)" | "Ragnasaur (Ashworld Critter)",
    96 => "Ursadon" | "Ursadon (Ice World)" | "Ursadon (Ice World Critter)",
    97 => "Lurker Egg" | "Zerg Lurker Egg",
    98 => "Raszagal" | "Raszagal (Corsair)",
    99 => "Samir Duran" | "Samir Duran (Ghost)",
    100 => "Alexei Stukov" | "Alexei Stukov (Ghost)",
    101 => "Map Revealer",
    102 => "Gerard DuGalle" | "Gerard DuGalle (Ghost)" | "Gerard DuGalle (BattleCruiser)",
    103 => "Zerg Lurker",
    104 => "Infested Duran" | "Infested Duran (Infested Terran)",
    105 => "Disruption Web" | "Disruption Field",
    106 => "Terran Command Center",
    107 => "Terran Comsat Station",
    108 => "Terran Nuclear Silo",
    109 => "Terran Supply Depot",
    110 => "Terran Refinery",
    111 => "Terran Barracks",
    112 => "Terran Academy",
    113 => "Terran Factory",
    114 => "Terran Starport",
    115 => "Terran Control Tower",
    116 => "Terran Science Facility",
    117 => "Terran Covert Ops",
    118 => "Terran Physics Lab",
    119 => "Starbase" | "Starbase (Unused)" | "Unused Terran Bldg type   1",
    120 => "Terran Machine Shop",
    121 => "Repair Bay" | "Repair Bay (Unused)" | "Unused Terran Bldg type   2",
    122 => "Terran Engineering Bay",
    123 => "Terran Armory",
    124 => "Terran Missile Turret",
    125 => "Terran Bunker",
    126 => "Norad II (Crashed)" | "Norad II (Crashed Battlecruiser)",
    127 => "Ion Cannon",
    128 => "Uraj Crystal",
    129 => "Khalis Crystal",
    130 => "Infested Command Center",
    131 => "Zerg Hatchery",
    132 => "Zerg Lair",
    133 => "Zerg Hive",
    134 => "Zerg Nydus Canal",
    135 => "Zerg Hydralisk Den",
    136 => "Zerg Defiler Mound",
    137 => "Zerg Greater Spire",
    138 => "Zerg Queen's Nest",
    139 => "Zerg Evolution Chamber",
    140 => "Zerg Ultralisk Cavern",
    141 => "Zerg Spire",
    142 => "Zerg Spawning Pool",
    143 => "Zerg Creep Colony",
    144 => "Zerg Spore Colony",
    145 => "Unused Zerg Bldg" | "Unused Zerg Building1",
    146 => "Zerg Sunken Colony",
    147 => "Zerg Overmind (With Shell)",
    148 => "Zerg Overmind",
    149 => "Zerg Extractor",
    150 => "Mature Chrysalis",
    151 => "Zerg Cerebrate",
    152 => "Zerg Cerebrate Daggoth",
    153 => "Unused Zerg Building2",
    154 => "Protoss Nexus",
    155 => "Protoss Robotics Facility",
    156 => "Protoss Pylon",
    157 => "Protoss Assimilator",
    158 => "Protoss Unused type   1" | "Unused Protoss Building1",
    159 => "Protoss Observatory",
    160 => "Protoss Gateway",
    161 => "Protoss Unused type   2" | "Unused Protoss Building2",
    162 => "Protoss Photon Cannon",
    163 => "Protoss Citadel of Adun",
    164 => "Protoss Cybernetics Core",
    165 => "Protoss Templar Archives",
    166 => "Protoss Forge",
    167 => "Protoss Stargate",
    168 => "Stasis Cell/Prison",
    169 => "Protoss Fleet Beacon",
    170 => "Protoss Arbiter Tribunal",
    171 => "Protoss Robotics Support Bay",
    172 => "Protoss Shield Battery",
    173 => "Khaydarin Crystal Formation",
    174 => "Protoss Temple",
    175 => "Xel'Naga Temple",
    176 => "Mineral Field (Type 1)",
    177 => "Mineral Field (Type 2)",
    178 => "Mineral Field (Type 3)",
    179 => "Cave" | "Cave (Unused)",
    180 => "Cave-in" | "Cave-in (Unused)",
    181 => "Cantina" | "Cantina (Unused)",
    182 => "Mining Platform" | "Mining Platform (Unused)",
    183 => "Independent Command Center" | "Independent Command Center (Unused)",
    184 => "Independent Starport" | "Independent Starport (Unused)",
    185 => "Jump Gate" | "Independent Jump Gate (Unused)",
    186 => "Ruins" | "Ruins (Unused)",
    187 => "Kyadarin Crystal Formation" | "Khaydarin Crystal Formation (Unused)",
    188 => "Vespene Geyser",
    189 => "Warp Gate",
    190 => "Psi Disrupter",
    191 => "Zerg Marker",
    192 => "Terran Marker",
    193 => "Protoss Marker",
    194 => "Zerg Beacon",
    195 => "Terran Beacon",
    196 => "Protoss Beacon",
    197 => "Zerg Flag Beacon",
    198 => "Terran Flag Beacon",
    199 => "Protoss Flag Beacon",
    200 => "Power Generator",
    201 => "Overmind Cocoon",
    202 => "Dark Swarm",
    203 => "Floor Missile Trap",
    204 => "Floor Hatch (Unused)" | "Floor Hatch (UNUSED)",
    205 => "Left Upper Level Door",
    206 => "Right Upper Level Door",
    207 => "Left Pit Door",
    208 => "Right Pit Door",
    209 => "Floor Gun Trap",
    210 => "Left Wall Missile Trap",
    211 => "Left Wall Flame Trap",
    212 => "Right Wall Missile Trap",
    213 => "Right Wall Flame Trap",
    214 => "Start Location",
    215 => "Flag",
    216 => "Young Chrysalis",
    217 => "Psi Emitter",
    218 => "Data Disc",
    219 => "Khaydarin Crystal",
    220 => "Mineral Chunk (Type 1)" | "Mineral Cluster Type 1",
    221 => "Mineral Chunk (Type 2)" | "Mineral Cluster Type 2",
    222 => "Vespene Orb (Protoss Type 1)" | "Protoss Vespene Gas Orb Type 1",
    223 => "Vespene Orb (Protoss Type 2)" | "Protoss Vespene Gas Orb Type 2",
    224 => "Vespene Sac (Zerg Type 1)" | "Zerg Vespene Gas Sac Type 1",
    225 => "Vespene Sac (Zerg Type 2)" | "Zerg Vespene Gas Sac Type 2",
    226 => "Vespene Tank (Terran Type 1)" | "Terran Vespene Gas Tank Type 1",
    227 => "Vespene Tank (Terran Type 2)" | "Terran Vespene Gas Tank Type 2",
}

/// Non-concrete stock unit names and unit-list filters (`228..=232`).
///
/// These are Canonical Symbols but are not part of the concrete 228-unit list.
const STOCK_UNIT_FILTERS: [CanonicalUnit; 5] = [
    CanonicalUnit {
        id: 228,
        name: "Unused unit 228",
        aliases: &["Unused228", "None"],
    },
    CanonicalUnit {
        id: 229,
        name: "Any unit",
        aliases: &["(any unit)"],
    },
    CanonicalUnit {
        id: 230,
        name: "Men",
        aliases: &["(men)"],
    },
    CanonicalUnit {
        id: 231,
        name: "Buildings",
        aliases: &["(buildings)"],
    },
    CanonicalUnit {
        id: 232,
        name: "Factories",
        aliases: &["(factories)"],
    },
];

/// The location ID StarCraft reserves for the `"Anywhere"` location.
pub const ANYWHERE_LOCATION_ID: u32 = 64;

/// One indexed view of the Canonical Symbol tables.
///
/// The struct is cheap to clone and carries no map state, so it can be built
/// and tested without loading an `.scx` file.
#[derive(Debug, Clone)]
pub struct CanonicalSymbols {
    unit_ids: HashMap<&'static str, u16>,
}

impl Default for CanonicalSymbols {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalSymbols {
    /// Builds the canonical name index from the one static table.
    pub fn new() -> Self {
        let mut unit_ids = HashMap::with_capacity(CANONICAL_UNITS.len() + STOCK_UNIT_FILTERS.len());
        for unit in CANONICAL_UNITS.iter().chain(STOCK_UNIT_FILTERS.iter()) {
            unit_ids.insert(unit.name, unit.id);
            for alias in unit.aliases {
                unit_ids.insert(alias, unit.id);
            }
        }
        Self { unit_ids }
    }

    /// Resolves a stock unit name or alias to its 0-based unit ID.
    pub fn unit_id(&self, name: &str) -> Option<u16> {
        self.unit_ids.get(name).copied()
    }

    /// Returns the canonical name for a concrete stock unit ID (`0..=227`).
    pub fn unit_name(&self, id: u16) -> Option<&'static str> {
        CANONICAL_UNIT_NAMES.get(id as usize).copied()
    }

    /// Returns the ordered concrete stock unit names (`0..=227`).
    pub fn unit_names(&self) -> &'static [&'static str] {
        &CANONICAL_UNIT_NAMES
    }

    /// Returns the full stock unit table, including aliases.
    pub fn units(&self) -> &'static [CanonicalUnit] {
        &CANONICAL_UNITS
    }

    /// Resolves a canonical location name (`"Anywhere"` or `"Location N"`).
    ///
    /// Map-defined location names belong to [`MapData`](crate::map_context::MapData).
    pub fn location_id(&self, name: &str) -> Option<u32> {
        if name == "Anywhere" {
            return Some(ANYWHERE_LOCATION_ID);
        }
        let rest = name.strip_prefix("Location ")?;
        let id = rest.parse::<u32>().ok()?;
        if (1..=255).contains(&id) && id != ANYWHERE_LOCATION_ID {
            Some(id)
        } else {
            None
        }
    }

    /// Resolves a canonical switch name (`"Switch N"`, 1-based).
    ///
    /// Map-defined switch names belong to [`MapData`](crate::map_context::MapData).
    pub fn switch_id(&self, name: &str) -> Option<u8> {
        let rest = name.strip_prefix("Switch ")?;
        let id = rest.parse::<u16>().ok()?;
        if (1..=256).contains(&id) {
            Some((id - 1) as u8)
        } else {
            None
        }
    }
}

/// Resolves a stock unit name or alias without constructing an index.
///
/// This is the compatibility entry point for callers that validate a single
/// name and do not hold a [`CanonicalSymbols`]. It reads the same static table.
pub fn stock_unit_id(name: &str) -> Option<u16> {
    CANONICAL_UNITS
        .iter()
        .chain(STOCK_UNIT_FILTERS.iter())
        .find(|unit| unit.name == name || unit.aliases.contains(&name))
        .map(|unit| unit.id)
}
